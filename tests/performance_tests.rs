// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
#![allow(unsafe_code)]
#![allow(unstable_features)]
//! Performance and stress tests for the submod CLI tool
//!
//! These tests verify that the tool performs well under various conditions
//! including multiple submodules, large repositories, and concurrent operations.

use std::fs;
use std::time::Instant;

mod common;
use common::TestHarness;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let size = layout.size();
            let prev = ALLOCATED.fetch_add(size, Ordering::Relaxed);
            let current = prev + size;
            loop {
                let peak = PEAK_ALLOCATED.load(Ordering::Relaxed);
                if current <= peak
                    || PEAK_ALLOCATED
                        .compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed)
                        .is_ok()
                {
                    break;
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static A: TrackingAllocator = TrackingAllocator;

fn reset_peak_memory() {
    let current = ALLOCATED.load(Ordering::Relaxed);
    PEAK_ALLOCATED.store(current, Ordering::Relaxed);
}

fn get_peak_memory() -> usize {
    PEAK_ALLOCATED.load(Ordering::Relaxed)
}

fn get_current_memory() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiple_submodules_performance() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let start_time = Instant::now();

        // Create multiple remote repositories
        let mut remote_repos = Vec::new();
        for i in 0..10 {
            let remote = harness
                .create_test_remote(&format!("perf_repo_{i}"))
                .expect("Failed to create remote");
            remote_repos.push(remote);
        }

        let setup_duration = start_time.elapsed();
        println!("Setup time for 10 remotes: {setup_duration:?}");

        // Add multiple submodules
        let add_start = Instant::now();
        for (i, remote_repo) in remote_repos.iter().enumerate() {
            let remote_url = format!("file://{}", remote_repo.display());
            harness
                .run_submod_success(&[
                    "add",
                    &remote_url,
                    "--name",
                    &format!("perf-submodule-{i}"),
                    "--path",
                    &format!("lib/perf{i}"),
                    "--sparse-paths",
                    "src,docs",
                ])
                .expect("Failed to add submodule");
        }

        let add_duration = add_start.elapsed();
        println!("Add time for 10 submodules: {add_duration:?}");

        // Test check performance
        let check_start = Instant::now();
        harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        let check_duration = check_start.elapsed();
        println!("Check time for 10 submodules: {check_duration:?}");

        // Test update performance
        let update_start = Instant::now();
        harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");
        let update_duration = update_start.elapsed();
        println!("Update time for 10 submodules: {update_duration:?}");

        // Performance assertions (these are rough guidelines)
        assert!(
            add_duration.as_secs() < 30,
            "Adding 10 submodules took too long: {add_duration:?}"
        );
        assert!(
            check_duration.as_secs() < 5,
            "Checking 10 submodules took too long: {check_duration:?}"
        );
        assert!(
            update_duration.as_secs() < 20,
            "Updating 10 submodules took too long: {update_duration:?}"
        );
    }

    #[test]
    fn test_large_config_file_performance() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a large config file with many submodules
        let mut large_config = String::from(
            r#"[defaults]
ignore = "dirty"
update = "checkout"

"#,
        );

        for i in 0..100 {
            use std::fmt::Write as _;
            let _ = write!(
                large_config,
                r#"[large-submodule-{i}]
path = "lib/large{i}"
url = "https://github.com/example/repo{i}.git"
active = true
sparse_paths = ["src/", "docs/", "include/"]
ignore = "all"

"#
            );
        }

        let config_start = Instant::now();
        harness
            .create_config(&large_config)
            .expect("Failed to create large config");
        let config_create_duration = config_start.elapsed();
        println!("Large config creation time: {config_create_duration:?}");

        // Test parsing performance
        let parse_start = Instant::now();
        harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        let parse_duration = parse_start.elapsed();
        println!("Large config parse time: {parse_duration:?}");

        // Performance assertions
        assert!(
            config_create_duration.as_millis() < 1000,
            "Config creation too slow: {config_create_duration:?}"
        );
        assert!(
            parse_duration.as_secs() < 2,
            "Config parsing too slow: {parse_duration:?}"
        );
    }

    #[test]
    fn test_deep_directory_structure() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("deep_structure")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Test with deep directory paths
        let deep_paths = vec![
            "level1/level2/level3/level4/level5",
            "a/very/deep/directory/structure/for/testing/performance",
            "deeply/nested/submodule/path/that/goes/many/levels/down",
        ];

        let start_time = Instant::now();
        for (i, deep_path) in deep_paths.iter().enumerate() {
            harness
                .run_submod_success(&[
                    "add",
                    &remote_url,
                    "--name",
                    &format!("deep-{i}"),
                    "--path",
                    deep_path,
                ])
                .expect("Failed to add deep submodule");
        }

        let duration = start_time.elapsed();
        println!("Deep directory creation time: {duration:?}");

        // Verify all were created successfully
        for deep_path in &deep_paths {
            assert!(harness.dir_exists(deep_path));
            assert!(harness.file_exists(&format!("{deep_path}/.git")));
        }

        // Performance assertion
        assert!(
            duration.as_secs() < 15,
            "Deep directory creation too slow: {duration:?}"
        );
    }

    #[test]
    fn test_sparse_checkout_with_many_patterns() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("many_patterns")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create a large number of sparse checkout patterns
        let mut patterns = Vec::new();
        for i in 0..50 {
            patterns.push(format!("src/module{i}/"));
            patterns.push(format!("docs/section{i}/"));
            patterns.push(format!("*.{i}"));
        }
        let pattern_string = patterns.join(",");

        let start_time = Instant::now();
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "many-patterns",
                "--path",
                "lib/many-patterns",
                "--sparse-paths",
                &pattern_string,
            ])
            .expect("Failed to add submodule with many patterns");

        let duration = start_time.elapsed();
        println!("Many patterns sparse checkout time: {duration:?}");

        // Verify sparse checkout was configured
        let sparse_file = harness.get_sparse_checkout_file_path("lib/many-patterns");
        assert!(sparse_file.exists());

        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");
        assert!(sparse_content.lines().count() >= 50);

        // Performance assertion
        assert!(
            duration.as_secs() < 10,
            "Many patterns processing too slow: {duration:?}"
        );
    }

    #[test]
    fn test_config_serialization_performance() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create multiple remotes for testing
        let mut remote_repos = Vec::new();
        for i in 0..20 {
            let remote = harness
                .create_test_remote(&format!("serial_repo_{i}"))
                .expect("Failed to create remote");
            remote_repos.push(remote);
        }

        // Add submodules one by one and measure config update performance
        let mut total_duration = std::time::Duration::new(0, 0);

        for (i, remote_repo) in remote_repos.iter().enumerate() {
            let remote_url = format!("file://{}", remote_repo.display());

            let start_time = Instant::now();
            harness
                .run_submod_success(&[
                    "add",
                    &remote_url,
                    "--name",
                    &format!("serial-{i}"),
                    "--path",
                    &format!("lib/serial{i}"),
                ])
                .expect("Failed to add submodule");

            let duration = start_time.elapsed();
            total_duration += duration;
            println!("Submodule {i} add time: {duration:?}");
        }

        println!("Total serialization time for 20 submodules: {total_duration:?}");

        // Verify final config integrity
        let final_config = harness.read_config().expect("Failed to read final config");
        let submodule_count = final_config
            .lines()
            .filter(|line| line.starts_with("[serial-"))
            .count();
        assert_eq!(submodule_count, 20);

        // Performance assertion
        assert!(
            total_duration.as_secs() < 60,
            "Config serialization too slow: {total_duration:?}"
        );
    }

    #[test]
    fn test_concurrent_check_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Set up multiple submodules
        for i in 0..5 {
            let remote_repo = harness
                .create_test_remote(&format!("concurrent_{i}"))
                .expect("Failed to create remote");
            let remote_url = format!("file://{}", remote_repo.display());

            harness
                .run_submod_success(&[
                    "add",
                    &remote_url,
                    "--name",
                    &format!("concurrent-{i}"),
                    "--path",
                    &format!("lib/concurrent{i}"),
                ])
                .expect("Failed to add submodule");
        }

        // Run multiple check operations concurrently using threads
        let start_time = Instant::now();
        let mut handles = Vec::new();
        let harness_arc = std::sync::Arc::new(harness);

        for _ in 0..10 {
            let h = harness_arc.clone();
            let handle = std::thread::spawn(move || {
                h.run_submod_success(&["check", "--verbose"])
                    .expect("Failed to run concurrent check");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        let duration = start_time.elapsed();

        println!("10 concurrent check operations time: {duration:?}");

        // Performance assertion
        assert!(
            duration.as_secs() < 30,
            "Concurrent checks too slow: {duration:?}"
        );
    }

    #[test]
    fn test_memory_usage_with_large_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a substantial number of submodules
        for i in 0..10 {
            let remote_repo = harness
                .create_test_remote(&format!("memory_test_{i}"))
                .expect("Failed to create remote");
            let remote_url = format!("file://{}", remote_repo.display());

            harness
                .run_submod_success(&[
                    "add",
                    &remote_url,
                    "--name",
                    &format!("memory-test-{i}"),
                    "--path",
                    &format!("lib/memory{i}"),
                    "--sparse-paths",
                    "src,docs,include,tests,examples",
                ])
                .expect("Failed to add submodule");
        }

        // Switch directory to the test workspace to run in-process
        let orig_dir = std::env::current_dir().expect("Failed to get current directory");
        std::env::set_current_dir(&harness.work_dir).expect("Failed to set CWD");

        reset_peak_memory();
        let mem_start = get_current_memory();
        let start_time = Instant::now();

        // Run comprehensive operations in-process
        let mut manager = submod::git_manager::GitManager::new(harness.config_path())
            .expect("Failed to load GitManager");

        manager.check_all_submodules().expect("Failed to check");

        let names: Vec<String> = manager
            .config()
            .get_submodules()
            .map(|(n, _)| n.clone())
            .collect();

        for name in &names {
            manager.update_submodule(name).expect("Failed to update");
        }

        // Run sync sequence in-process
        manager.check_all_submodules().expect("Failed to check");
        for name in &names {
            manager.init_submodule(name).expect("Failed to init");
            manager.update_submodule(name).expect("Failed to update");
        }

        let duration = start_time.elapsed();
        let peak_mem = get_peak_memory();
        let net_peak = peak_mem.saturating_sub(mem_start);

        // Restore working directory
        std::env::set_current_dir(orig_dir).ok();

        println!("Large operations in-process time: {duration:?}");
        println!(
            "Peak memory usage during large operations: {} KB",
            net_peak / 1024
        );

        // Assert memory usage is within reasonable bounds (e.g. less than 20 MB)
        assert!(
            net_peak < 20 * 1024 * 1024,
            "Peak memory usage too high: {net_peak} bytes"
        );

        // If we reach here without OOM or crashes, the test passes
        assert!(
            duration.as_secs() < 60,
            "Large operations too slow: {duration:?}"
        );
    }

    #[test]
    fn test_file_system_performance() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("fs_perf")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Test operations that involve heavy file system access
        let start_time = Instant::now();

        // Add submodule
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "fs-perf",
                "--path",
                "lib/fs-perf",
                "--sparse-paths",
                "src,docs,tests,examples",
            ])
            .expect("Failed to add submodule");

        // Run reset (heavy FS operations)
        harness
            .run_submod_success(&["reset", "fs-perf"])
            .expect("Failed to reset submodule");

        // Run check (FS scanning)
        harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");

        let duration = start_time.elapsed();
        println!("File system heavy operations time: {duration:?}");

        // Performance assertion
        assert!(
            duration.as_secs() < 20,
            "FS operations too slow: {duration:?}"
        );
    }

    #[test]
    fn test_config_with_unicode_and_special_chars() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let unicode_config = r#"["测试-submodule"]
path = "lib/测试"
url = "https://github.com/用户/项目.git"
active = true
sparse_paths = ["源码/", "文档/", "*.md"]

["émoji-test-🚀"]
path = "lib/émoji🚀"
url = "https://github.com/user/émoji-repo.git"
active = true

["special-chars-!@#$%"]
path = "lib/special"
url = "https://github.com/user/special-chars.git"
active = true
"#;

        let start_time = Instant::now();
        harness
            .create_config(unicode_config)
            .expect("Failed to create unicode config");

        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        let duration = start_time.elapsed();

        println!("Unicode config processing time: {duration:?}");
        assert!(stdout.contains("Checking submodule configurations"));

        // Performance assertion
        assert!(
            duration.as_millis() < 2000,
            "Unicode processing too slow: {duration:?}"
        );
    }

    #[test]
    fn test_lock_contention_handling() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("lock_test")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "lock-sub",
                "--path",
                "lib/lock-sub",
            ])
            .expect("Failed to add submodule");

        // Manually create .git/index.lock in the superproject to simulate a lock.
        let super_lock_path = harness.work_dir.join(".git").join("index.lock");
        fs::write(&super_lock_path, "locked").expect("Failed to create superproject lock file");

        // Running an operation that modifies the superproject (like adding a new submodule)
        // should fail gracefully due to the locked index.
        let remote_repo2 = harness
            .create_test_remote("lock_test2")
            .expect("Failed to create remote 2");
        let remote_url2 = format!("file://{}", remote_repo2.display());

        let output = harness
            .run_submod(&[
                "add",
                &remote_url2,
                "--name",
                "lock-sub2",
                "--path",
                "lib/lock-sub2",
            ])
            .expect("Failed to run submod");

        assert!(
            !output.status.success(),
            "Command should fail when index is locked"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("index.lock") || stderr.contains("lock") || stderr.contains("failed"),
            "Error message should mention lock or failure, got: {stderr}"
        );

        // Remove the lock
        fs::remove_file(&super_lock_path).expect("Failed to remove lock file");

        // Now it should succeed
        harness
            .run_submod_success(&[
                "add",
                &remote_url2,
                "--name",
                "lock-sub2",
                "--path",
                "lib/lock-sub2",
            ])
            .expect("Failed to add submodule after lock release");

        // Lock the submodule's index
        let sub_git_dir = harness
            .get_sparse_checkout_file_path("lib/lock-sub")
            .parent() // info
            .unwrap()
            .parent() // gitdir root
            .unwrap()
            .to_path_buf();
        let sub_lock_path = sub_git_dir.join("index.lock");
        fs::write(&sub_lock_path, "locked").expect("Failed to create submodule lock file");

        // Run reset which modifies the submodule index, and verify failure
        let output2 = harness
            .run_submod(&["reset", "lock-sub"])
            .expect("Failed to run reset");
        assert!(
            !output2.status.success(),
            "Reset should fail when submodule index is locked"
        );

        // Clean up lock
        fs::remove_file(&sub_lock_path).expect("Failed to remove submodule lock");
    }
}
