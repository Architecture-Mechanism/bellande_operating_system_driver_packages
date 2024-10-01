// Copyright (C) 2024 Bellande Architecture Mechanism Research Innovation Center, Ronaldson Bellande

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use assert_cmd::Command as TestCommand;
use predicates::prelude::*;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::TempDir;

const BELLANDE_FS_BINARY: &str = "./bellandeos/file_system";

struct TestContext {
    temp_dir: TempDir,
    device_path: PathBuf,
}

impl TestContext {
    fn new() -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let device_path = temp_dir.path().join("test_device");

        // Create a test device file
        let mut file = File::create(&device_path)?;
        let zeros = vec![0u8; 1024 * 1024]; // 1MB device
        file.write_all(&zeros)?;

        Ok(TestContext {
            temp_dir,
            device_path,
        })
    }

    fn run_bellande_command(&self, args: &[&str]) -> assert_cmd::Command {
        let mut cmd = TestCommand::cargo_bin(BELLANDE_FS_BINARY).unwrap();
        cmd.arg("--device").arg(&self.device_path);
        cmd.args(args);
        cmd
    }
}

// Define the actual test functions without the #[test] attribute
fn format_device(ctx: &TestContext) -> io::Result<()> {
    ctx.run_bellande_command(&["format"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Device formatted successfully"));
    Ok(())
}

fn create_and_list_files(ctx: &TestContext) -> io::Result<()> {
    // Format device
    format_device(ctx)?;

    // Create a file
    ctx.run_bellande_command(&["create", "--path", "/test.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("File created successfully"));

    // List files
    ctx.run_bellande_command(&["list", "--path", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
    Ok(())
}

fn write_and_read_file(ctx: &TestContext) -> io::Result<()> {
    let test_content = "Hello, Bellande filesystem!";
    let test_file = "/test.txt";

    format_device(ctx)?;

    ctx.run_bellande_command(&["create", "--path", test_file])
        .assert()
        .success();

    ctx.run_bellande_command(&["write", "--path", test_file])
        .write_stdin(test_content)
        .assert()
        .success();

    ctx.run_bellande_command(&["read", "--path", test_file])
        .assert()
        .success()
        .stdout(predicate::str::contains(test_content));
    Ok(())
}

fn create_and_remove_directory(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;

    ctx.run_bellande_command(&["mkdir", "--path", "/testdir"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Directory created successfully"));

    ctx.run_bellande_command(&["list", "--path", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("testdir"));

    ctx.run_bellande_command(&["rmdir", "--path", "/testdir"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Directory removed successfully"));

    // Fixed line:
    ctx.run_bellande_command(&["list", "--path", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("testdir").not());
    Ok(())
}

fn filesystem_stats(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;

    ctx.run_bellande_command(&["stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total blocks"))
        .stdout(predicate::str::contains("Free blocks"))
        .stdout(predicate::str::contains("Total inodes"))
        .stdout(predicate::str::contains("Free inodes"));
    Ok(())
}

fn error_handling(ctx: &TestContext) -> io::Result<()> {
    // Try to use unformatted device first
    ctx.run_bellande_command(&["list", "--path", "/"])
        .assert()
        .failure();

    format_device(ctx)?;

    ctx.run_bellande_command(&["remove", "--path", "/nonexistent.txt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("File not found"));

    ctx.run_bellande_command(&["create", "--path", "invalid/path/file.txt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid path"));
    Ok(())
}

fn large_file_operations(ctx: &TestContext) -> io::Result<()> {
    let large_content = "A".repeat(100_000);

    format_device(ctx)?;

    ctx.run_bellande_command(&["create", "--path", "/large.txt"])
        .assert()
        .success();

    ctx.run_bellande_command(&["write", "--path", "/large.txt"])
        .write_stdin(large_content.as_bytes())
        .assert()
        .success();

    ctx.run_bellande_command(&["read", "--path", "/large.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&large_content));
    Ok(())
}

// Define test module with #[test] functions that call the actual test functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_device() -> io::Result<()> {
        let ctx = TestContext::new()?;
        format_device(&ctx)
    }

    #[test]
    fn test_create_and_list_files() -> io::Result<()> {
        let ctx = TestContext::new()?;
        create_and_list_files(&ctx)
    }

    #[test]
    fn test_write_and_read_file() -> io::Result<()> {
        let ctx = TestContext::new()?;
        write_and_read_file(&ctx)
    }

    #[test]
    fn test_create_and_remove_directory() -> io::Result<()> {
        let ctx = TestContext::new()?;
        create_and_remove_directory(&ctx)
    }

    #[test]
    fn test_filesystem_stats() -> io::Result<()> {
        let ctx = TestContext::new()?;
        filesystem_stats(&ctx)
    }

    #[test]
    fn test_error_handling() -> io::Result<()> {
        let ctx = TestContext::new()?;
        error_handling(&ctx)
    }

    #[test]
    fn test_large_file_operations() -> io::Result<()> {
        let ctx = TestContext::new()?;
        large_file_operations(&ctx)
    }
}

fn run_full_test_suite(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;
    create_and_list_files(ctx)?;
    write_and_read_file(ctx)?;
    create_and_remove_directory(ctx)?;
    filesystem_stats(ctx)?;
    error_handling(ctx)?;
    large_file_operations(ctx)?;
    Ok(())
}

fn main() -> io::Result<()> {
    println!("Running Bellande filesystem integration tests...");
    let ctx = TestContext::new()?;
    match run_full_test_suite(&ctx) {
        Ok(_) => println!("All tests passed successfully!"),
        Err(e) => {
            eprintln!("Test suite failed: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}
