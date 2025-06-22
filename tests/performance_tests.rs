//! Performance and stress tests for the submod CLI tool
//!
//! These tests verify that the tool performs well under various conditions
//! including multiple submodules, large repositories, and concurrent operations.

use std::fs;
use std::time::Instant;

mod common;
use common::TestHarness;

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
                    &format!("perf-submodule-{i}"),
                    &format!("lib/perf{i}"),
                    &remote_url,
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
            .run_submod_success(&["check"])
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
            large_config.push_str(&format!(
                r#"[large-submodule-{i}]
path = "lib/large{i}"
url = "https://github.com/example/repo{i}.git"
active = true
sparse_paths = ["src/", "docs/", "include/"]
ignore = "all"

"#
            ));
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
            .run_submod_success(&["check"])
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
                .run_submod_success(&["add", &format!("deep-{i}"), deep_path, &remote_url])
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
                "many-patterns",
                "lib/many-patterns",
                &remote_url,
                "--sparse-paths",
                &pattern_string,
            ])
            .expect("Failed to add submodule with many patterns");

        let duration = start_time.elapsed();
        println!("Many patterns sparse checkout time: {duration:?}");

        // Verify sparse checkout was configured
        let sparse_file = harness
            .work_dir
            .join("lib/many-patterns/.git/info/sparse-checkout");
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
                    &format!("serial-{i}"),
                    &format!("lib/serial{i}"),
                    &remote_url,
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
                    &format!("concurrent-{i}"),
                    &format!("lib/concurrent{i}"),
                    &remote_url,
                ])
                .expect("Failed to add submodule");
        }

        // Run multiple check operations rapidly
        let start_time = Instant::now();
        for _ in 0..10 {
            harness
                .run_submod_success(&["check"])
                .expect("Failed to run check");
        }
        let duration = start_time.elapsed();

        println!("10 consecutive check operations time: {duration:?}");

        // Performance assertion
        assert!(
            duration.as_secs() < 30,
            "Consecutive checks too slow: {duration:?}"
        );
    }

    #[test]
    fn test_memory_usage_with_large_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // This test is more about ensuring operations complete without memory issues
        // rather than measuring actual memory usage

        // Create a substantial number of submodules
        for i in 0..25 {
            let remote_repo = harness
                .create_test_remote(&format!("memory_test_{i}"))
                .expect("Failed to create remote");
            let remote_url = format!("file://{}", remote_repo.display());

            harness
                .run_submod_success(&[
                    "add",
                    &format!("memory-test-{i}"),
                    &format!("lib/memory{i}"),
                    &remote_url,
                    "--sparse-paths",
                    "src,docs,include,tests,examples",
                ])
                .expect("Failed to add submodule");
        }

        // Run comprehensive operations
        let start_time = Instant::now();

        harness
            .run_submod_success(&["check"])
            .expect("Failed to run check");
        harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");
        harness
            .run_submod_success(&["sync"])
            .expect("Failed to run sync");

        let duration = start_time.elapsed();
        println!("Large operations time: {duration:?}");

        // If we reach here without OOM or crashes, the test passes
        assert!(
            duration.as_secs() < 120,
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
                "fs-perf",
                "lib/fs-perf",
                &remote_url,
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
            .run_submod_success(&["check"])
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

        let unicode_config = r#"[测试-submodule]
path = "lib/测试"
url = "https://github.com/用户/项目.git"
active = true
sparse_paths = ["源码/", "文档/", "*.md"]

[émoji-test-🚀]
path = "lib/émoji🚀"
url = "https://github.com/user/émoji-repo.git"
active = true

[special-chars-!@#$%]
path = "lib/special"
url = "https://github.com/user/special-chars.git"
active = true
"#;

        let start_time = Instant::now();
        harness
            .create_config(unicode_config)
            .expect("Failed to create unicode config");

        let stdout = harness
            .run_submod_success(&["check"])
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
}
