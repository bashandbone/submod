#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Adam Poulemanos
# SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
"""Before/after performance comparison for Phase 7 (R28).

Compares two immutable CLI executables (baseline vs candidate, same
``--profile``) plus two criterion benchmark executables on real production
workloads:

* config parse / load-from-file / add-one at 1/10/100 modules, driven through
  the ``SUBMOD_MEASURE_CONFIG`` mode of ``benches/benchmark.rs`` (1000 timed
  iterations per sample, preparation and validation outside the clock);
* CLI ``check --verbose`` on declared-but-absent modules (inspection only);
* CLI ``check --verbose`` then ``sync`` on small materialized fixtures, with
  byte/mtime/index snapshots proving the no-op path performs no clone, fetch,
  sparse reapply, or config rewrite.

Method (see docs/IMPROVEMENT_PLAN.md Phase 7 and the measurement protocol):
one untimed validation, one warm-up, then alternating baseline/candidate wall
clock samples (default 10) on the same host/filesystem with quiesced builds.
A separate instrumented sample per artifact/workload counts direct Git
invocations (PATH wrapper log) and Git-spawned launches (Trace2
``child_start`` events); those counts are reported separately, never summed.
Memory is direct-executable accounting only (``/usr/bin/time -l`` on macOS,
``/usr/bin/time -v`` on Linux); unavailable child RSS is reported as missing,
never zero.

Usage:
    python3 scripts/measure-performance.py \\
        --baseline /path/to/phase6-submod --candidate /path/to/phase7-submod \\
        --bench-baseline /path/to/phase6-benchmark \\
        --bench-candidate /path/to/phase7-benchmark \\
        --profile bench --samples 10 --workdir /tmp/submod-perf

Build the executables first with the identical profile, e.g.
``cargo build --locked --offline --profile bench`` (CLI) and
``cargo bench --locked --offline --no-run --profile bench`` (criterion), then
copy each to an immutable path before building the other revision.
"""

import argparse
import hashlib
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ITERATIONS = 1000


def run(cmd, **kwargs):
    kwargs.setdefault("check", True)
    kwargs.setdefault("text", True)
    kwargs.setdefault("capture_output", True)
    return subprocess.run(cmd, **kwargs)


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def isolate_git_env(workdir, extra=None):
    """Per-run Git isolation mirroring tests/common/mod.rs conventions."""
    gitconfig = os.path.join(workdir, "fixture-gitconfig")
    if not os.path.exists(gitconfig):
        with open(gitconfig, "w") as handle:
            handle.write(
                '[protocol "file"]\n\tallow = always\n'
                "[core]\n\tautocrlf = false\n\tfilemode = false\n"
                "[commit]\n\tgpgsign = false\n"
                "[user]\n\tname = Perf Measure\n\temail = perf@example.com\n"
            )
    env = dict(os.environ)
    env["GIT_CONFIG_GLOBAL"] = gitconfig
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_TERMINAL_PROMPT"] = "0"
    if extra:
        env.update(extra)
    return env


def git(env, cwd, *args):
    return run(["git", *args], cwd=cwd, env=env)


def make_origin(root, name, branch="main"):
    """Bare origin with two commits; returns (path, pinned_oid, tree_oid)."""
    origin = os.path.join(root, f"{name}.git")
    work = os.path.join(root, f"{name}-work")
    os.makedirs(work)
    env = isolate_git_env(root)
    git(env, work, "init", "-b", branch, ".")
    with open(os.path.join(work, "file.txt"), "w") as handle:
        handle.write(f"{name} one\n")
    git(env, work, "add", "file.txt")
    git(env, work, "commit", "-m", "one")
    with open(os.path.join(work, "file.txt"), "a") as handle:
        handle.write(f"{name} two\n")
    git(env, work, "commit", "-am", "two")
    pinned = git(env, work, "rev-parse", "HEAD").stdout.strip()
    tree = git(env, work, "rev-parse", "HEAD^{tree}").stdout.strip()
    git(env, root, "clone", "--bare", work, origin)
    return origin, pinned, tree


def write_config_toml(path, count, origins, sparse=True, defaults=True):
    # CLI fixtures omit [defaults]: `git submodule add` writes only path+url,
    # so inherited defaults would read as metadata drift. Config-bench
    # datasets keep defaults for parse realism.
    lines = (
        ['[defaults]\nignore = "dirty"\nupdate = "checkout"\n'] if defaults else []
    )
    for i in range(count):
        entry = (
            f'[module-{i}]\npath = "lib/module-{i}"\nurl = "{origins[i]}"\n'
            "active = true\n"
        )
        if sparse:
            entry += 'sparse_paths = ["src", "docs"]\n'
        lines.append(entry)
    with open(path, "w") as handle:
        handle.write("\n".join(lines) + "\n")
    return sha256_file(path)


def materialize_parent(root, name, count):
    """Parent repo with `count` submodules added via real Git; committed."""
    env = isolate_git_env(root)
    parent = os.path.join(root, name)
    os.makedirs(parent)
    git(env, parent, "init", "-b", "main", ".")
    origins = []
    pins = []
    for i in range(count):
        origin, pinned, _tree = make_origin(root, f"{name}-origin-{i}")
        origins.append(origin)
        pins.append(pinned)
        git(
            env,
            parent,
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--name",
            f"module-{i}",
            origin,
            f"lib/module-{i}",
        )
    # No sparse declaration and no defaults: fixtures are materialized as full
    # checkouts via plain `git submodule add` (path+url only), so the no-op
    # workloads isolate pin convergence, not metadata/sparse drift.
    with open(os.path.join(parent, "submod.toml"), "w") as handle:
        for i, origin in enumerate(origins):
            handle.write(
                f'[module-{i}]\npath = "lib/module-{i}"\nurl = "{origin}"\nactive = true\n'
            )
    git(env, parent, "add", "-A")
    git(env, parent, "commit", "-m", "fixture")
    manifest = {
        "modules": count,
        "pins": pins,
        "toml_sha256": sha256_file(os.path.join(parent, "submod.toml")),
    }
    return parent, manifest


def snapshot_state(cli_env, repo):
    """File bytes + mtimes, index listing, and porcelain status."""
    files = {}
    for dirpath, dirnames, filenames in os.walk(repo):
        dirnames[:] = [d for d in dirnames if d != ".git"]
        for filename in filenames:
            full = os.path.join(dirpath, filename)
            rel = os.path.relpath(full, repo)
            with open(full, "rb") as handle:
                digest = hashlib.sha256(handle.read()).hexdigest()
            files[rel] = (digest, os.path.getmtime(full))
    index = run(
        ["git", "ls-files", "--stage"], cwd=repo, env=cli_env,
        check=True, text=True, capture_output=True,
    ).stdout
    status = run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=repo, env=cli_env, check=True, text=True, capture_output=True,
    ).stdout
    return {"files": files, "index": index, "status": status}


def time_cli(cli, cli_env, repo, args):
    start = time.monotonic_ns()
    proc = run([cli, *args], cwd=repo, env=cli_env, check=False)
    elapsed_ns = time.monotonic_ns() - start
    return proc.returncode, elapsed_ns, proc.stdout, proc.stderr


def make_git_wrapper(bin_dir, log_path):
    """PATH shim logging every direct `git` spawn (argv, tab-separated)."""
    real_git = shutil.which("git")
    shim = os.path.join(bin_dir, "git")
    with open(shim, "w") as handle:
        handle.write(
            "#!/bin/sh\n"
            f'printf "%s\\n" "$*" >> {log_path}\n'
            f'exec {real_git} "$@"\n'
        )
    os.chmod(shim, 0o755)
    return real_git


def count_trace2_children(event_path):
    """Count Trace2 child_start events: Git-spawned launches, not an OS-wide
    process census. Reported separately from direct invocations."""
    count = 0
    try:
        with open(event_path) as handle:
            for line in handle:
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if event.get("event") == "child_start":
                    count += 1
    except FileNotFoundError:
        return None
    return count


def instrumented_cli_sample(cli, repo, args, workdir):
    """One instrumented run: direct git count, Trace2 child launches,
    no-op state identity, and direct-executable resource accounting."""
    run_id = f"instr-{os.getpid()}-{time.monotonic_ns()}"
    bin_dir = os.path.join(workdir, f"shim-{run_id}")
    os.makedirs(bin_dir)
    git_log = os.path.join(workdir, f"gitcalls-{run_id}.log")
    open(git_log, "w").close()
    make_git_wrapper(bin_dir, git_log)
    trace_path = os.path.join(workdir, f"trace2-{run_id}.json")
    env = isolate_git_env(workdir)
    env["PATH"] = bin_dir + os.pathsep + env.get("PATH", "")
    env["GIT_TRACE2_EVENT"] = trace_path
    env["GIT_TRACE2_EVENT_NESTING"] = "5"

    before = snapshot_state(env, repo)
    start = time.monotonic_ns()
    proc = run([cli, *args], cwd=repo, env=env, check=False)
    elapsed_ns = time.monotonic_ns() - start
    after = snapshot_state(env, repo)

    with open(git_log) as handle:
        direct_git_calls = sum(1 for line in handle if line.strip())
    traced_launches = count_trace2_children(trace_path)
    state_unchanged = before == after

    memory = {"method": "unavailable", "detail": "no supported accounting tool"}
    if sys.platform == "darwin" and shutil.which("/usr/bin/time"):
        timed = run(
            ["/usr/bin/time", "-l", cli, *args], cwd=repo, env=env, check=False,
        )
        memory = {
            "method": "/usr/bin/time -l (direct executable only; "
            "excludes concurrently running Git children)",
            "raw_stderr_tail": timed.stderr[-2000:],
        }
    elif sys.platform.startswith("linux") and shutil.which("/usr/bin/time"):
        timed = run(
            ["/usr/bin/time", "-v", cli, *args], cwd=repo, env=env, check=False,
        )
        memory = {
            "method": "/usr/bin/time -v (direct executable only)",
            "raw_stderr_tail": timed.stderr[-2000:],
        }
    return {
        "exit": proc.returncode,
        "elapsed_ns": elapsed_ns,
        "direct_git_calls": direct_git_calls,
        "traced_git_child_launches": traced_launches,
        "state_unchanged": state_unchanged,
        "memory": memory,
    }


def bench_config_sample(bench_bin, workload, count, toml_path):
    """One SUBMOD_MEASURE_CONFIG sample: 1000 timed iterations in-process."""
    env = dict(os.environ)
    env["SUBMOD_MEASURE_CONFIG"] = workload
    env["SUBMOD_MEASURE_COUNT"] = str(count)
    env["SUBMOD_MEASURE_FILE"] = toml_path
    start = time.monotonic_ns()
    proc = run([bench_bin], env=env, check=False)
    wall_ns = time.monotonic_ns() - start
    try:
        payload = json.loads(proc.stdout.strip().splitlines()[-1])
    except (IndexError, json.JSONDecodeError):
        payload = {"verdict": "unparsable", "raw": proc.stdout[-500:]}
    payload["process_exit"] = proc.returncode
    payload["wall_ns"] = wall_ns
    return payload


def alternating(samples, baseline_fn, candidate_fn):
    """Alternate baseline/candidate samples; returns (base_list, cand_list)."""
    base, cand = [], []
    for _ in range(samples):
        base.append(baseline_fn())
        cand.append(candidate_fn())
    return base, cand


def summarize(ns_list):
    return {
        "samples": len(ns_list),
        "median_ns": int(statistics.median(ns_list)),
        "min_ns": min(ns_list),
        "max_ns": max(ns_list),
    }


def main():
    parser = argparse.ArgumentParser(description="Phase 7 before/after measurement")
    parser.add_argument("--baseline", required=True, help="immutable baseline CLI")
    parser.add_argument("--candidate", required=True, help="immutable candidate CLI")
    parser.add_argument("--bench-baseline", required=True, help="immutable baseline criterion executable")
    parser.add_argument("--bench-candidate", required=True, help="immutable candidate criterion executable")
    parser.add_argument("--profile", default="bench")
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--workdir", required=True)
    parser.add_argument("--out", default="-", help="JSONL output path, - for stdout")
    args = parser.parse_args()

    for path in (args.baseline, args.candidate, args.bench_baseline, args.bench_candidate):
        if not (os.path.isfile(path) and os.access(path, os.X_OK)):
            parser.error(f"not an executable file: {path}")
    os.makedirs(args.workdir, exist_ok=True)
    artifacts = {
        "baseline": {"cli": args.baseline, "bench": args.bench_baseline},
        "candidate": {"cli": args.candidate, "bench": args.bench_candidate},
    }
    identity = {}
    for name, paths in artifacts.items():
        identity[name] = {
            "cli_sha256": sha256_file(paths["cli"]),
            "cli_bytes": os.path.getsize(paths["cli"]),
            "bench_sha256": sha256_file(paths["bench"]),
        }
    git_version = run(["git", "--version"], check=True, text=True, capture_output=True).stdout.strip()
    header = {
        "record": "header",
        "profile": args.profile,
        "samples_per_workload": args.samples,
        "host": {
            "platform": sys.platform,
            "git": git_version,
            "python": sys.version.split()[0],
        },
        "artifacts": identity,
    }

    records = [header]
    fixture_root = tempfile.mkdtemp(prefix="submod-perf-", dir=args.workdir)

    # --- Config datasets (1/10/100) shared by both bench executables ---
    datasets = {}
    for count in (1, 10, 100):
        origins = [f"file:///local/origin-{count}-{i}" for i in range(count)]
        toml_path = os.path.join(fixture_root, f"config-{count}.toml")
        toml_sha = write_config_toml(toml_path, count, origins)
        datasets[count] = {"toml": toml_path, "sha256": toml_sha}

    for workload in ("parse", "load", "add"):
        for count, dataset in datasets.items():
            # Untimed validation on both artifacts first.
            for name in ("baseline", "candidate"):
                check = bench_config_sample(
                    artifacts[name]["bench"], workload, count, dataset["toml"]
                )
                if check.get("verdict") != "pass" or check["process_exit"] != 0:
                    raise SystemExit(
                        f"validation failed: {name} {workload}@{count}: {check}"
                    )
            # Warm-up (labeled, discarded), then alternating timed samples.
            bench_config_sample(
                artifacts["candidate"]["bench"], workload, count, dataset["toml"]
            )
            base, cand = alternating(
                args.samples,
                lambda: bench_config_sample(
                    artifacts["baseline"]["bench"], workload, count, dataset["toml"]
                ),
                lambda: bench_config_sample(
                    artifacts["candidate"]["bench"], workload, count, dataset["toml"]
                ),
            )
            for name, samples in (("baseline", base), ("candidate", cand)):
                inner = [s["elapsed_ns"] for s in samples]
                records.append({
                    "record": "config",
                    "artifact": name,
                    "artifact_sha256": identity[name]["bench_sha256"],
                    "workload": workload,
                    "modules": count,
                    "dataset_sha256": dataset["sha256"],
                    "iterations_per_sample": ITERATIONS,
                    "verdict": "pass"
                    if all(s.get("verdict") == "pass" for s in samples)
                    else "FAIL",
                    "inner_timing_ns": summarize(inner),
                    "outer_wall_ns": summarize([s["wall_ns"] for s in samples]),
                })

    # --- CLI fixtures: absent-module inspection + materialized no-op sync ---
    cli_env = isolate_git_env(fixture_root)
    absent_repos = {}
    for count in (1, 10, 100):
        repo = os.path.join(fixture_root, f"absent-{count}")
        os.makedirs(repo)
        git(cli_env, repo, "init", "-b", "main", ".")
        git(cli_env, repo, "commit", "--allow-empty", "-m", "init")
        origins = [f"/nonexistent/origin-{count}-{i}" for i in range(count)]
        toml = os.path.join(repo, "submod.toml")
        # No sparse declaration and no defaults: this workload isolates
        # missing-checkout inspection, not metadata/sparse drift.
        toml_sha = write_config_toml(toml, count, origins, sparse=False, defaults=False)
        absent_repos[count] = {"repo": repo, "toml_sha256": toml_sha}

    materialized = {}
    for count in (1, 10):
        repo, manifest = materialize_parent(fixture_root, f"mat-{count}", count)
        materialized[count] = {"repo": repo, "manifest": manifest}

    cli_workloads = []
    for count, fix in absent_repos.items():
        cli_workloads.append({
            "name": "check-absent", "modules": count, "repo": fix["repo"],
            "args": ["check", "--verbose"], "expect_exit": 1,
            "dataset": fix["toml_sha256"], "must_state_hold": False,
        })
    for count, fix in materialized.items():
        cli_workloads.append({
            "name": "check-materialized", "modules": count, "repo": fix["repo"],
            "args": ["check", "--verbose"], "expect_exit": 0,
            "dataset": fix["manifest"], "must_state_hold": False,
        })
        cli_workloads.append({
            "name": "sync-noop", "modules": count, "repo": fix["repo"],
            "args": ["sync"], "expect_exit": 0,
            "dataset": fix["manifest"], "must_state_hold": True,
        })

    for workload in cli_workloads:
        # Untimed validation on both artifacts.
        for name in ("baseline", "candidate"):
            code, _ns, _out, _err = time_cli(
                artifacts[name]["cli"], cli_env, workload["repo"], workload["args"]
            )
            if code != workload["expect_exit"]:
                raise SystemExit(
                    f"validation failed: {name} {workload['name']}@{workload['modules']}: "
                    f"exit {code}, expected {workload['expect_exit']}"
                )
        if workload["must_state_hold"]:
            before = snapshot_state(cli_env, workload["repo"])
        # Warm-up, then alternating samples on pristine state. CLI check/sync
        # are read-only/no-op here, so no restore is needed between samples;
        # any unexpected mutation fails the state assertion below.
        time_cli(artifacts["candidate"]["cli"], cli_env, workload["repo"], workload["args"])
        base, cand = alternating(
            args.samples,
            lambda: time_cli(artifacts["baseline"]["cli"], cli_env, workload["repo"], workload["args"]),
            lambda: time_cli(artifacts["candidate"]["cli"], cli_env, workload["repo"], workload["args"]),
        )
        verdict = "pass"
        for samples in (base, cand):
            for code, _ns, _out, _err in samples:
                if code != workload["expect_exit"]:
                    verdict = "FAIL"
        state_held = None
        if workload["must_state_hold"]:
            after = snapshot_state(cli_env, workload["repo"])
            state_held = before == after
            if not state_held:
                verdict = "FAIL"
        for name, samples in (("baseline", base), ("candidate", cand)):
            records.append({
                "record": "cli",
                "artifact": name,
                "artifact_sha256": identity[name]["cli_sha256"],
                "workload": workload["name"],
                "modules": workload["modules"],
                "dataset": workload["dataset"],
                "expected_exit": workload["expect_exit"],
                "verdict": verdict,
                "no_op_state_held": state_held,
                "wall_ns": summarize([ns for _c, ns, _o, _e in samples]),
            })
        # One instrumented sample per artifact (diagnostic, not timed).
        for name in ("baseline", "candidate"):
            sample = instrumented_cli_sample(
                artifacts[name]["cli"], workload["repo"], workload["args"], fixture_root
            )
            records.append({
                "record": "instrumented",
                "artifact": name,
                "artifact_sha256": identity[name]["cli_sha256"],
                "workload": workload["name"],
                "modules": workload["modules"],
                **sample,
            })

    out = sys.stdout if args.out == "-" else open(args.out, "w")
    with out:
        for record in records:
            out.write(json.dumps(record, sort_keys=True) + "\n")
    print(
        f"wrote {len(records)} records for "
        f"{len(datasets) * 3} config + {len(cli_workloads)} CLI workloads",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()

