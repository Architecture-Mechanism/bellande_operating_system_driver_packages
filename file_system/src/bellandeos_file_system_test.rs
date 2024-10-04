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

use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

#[cfg(test)]
use predicates::prelude::*;

fn get_bellande_fs_binary() -> PathBuf {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {:?}", current_dir);

    let mut path = current_dir;
    path.push("bellandeos");
    path.push("file_system");

    println!("Constructed binary path: {:?}", path);
    path
}

struct TestContext {
    temp_dir: TempDir,
    device_path: PathBuf,
    binary_path: PathBuf,
}

impl TestContext {
    fn new() -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let device_path = temp_dir.path().join("test_device");
        let binary_path = get_bellande_fs_binary();

        // Create a larger test device file (e.g., 10MB)
        let mut file = File::create(&device_path)?;
        let zeros = vec![0u8; 10 * 1024 * 1024];
        file.write_all(&zeros)?;

        Ok(TestContext {
            temp_dir,
            device_path,
            binary_path,
        })
    }

    fn run_bellande_command(&self, args: &[&str]) -> io::Result<Output> {
        let mut command = Command::new(&self.binary_path);
        command.arg("--device").arg(&self.device_path).args(args);

        println!("Executing command: {:?}", command);

        let output = command.output()?;

        println!(
            "Command stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        println!(
            "Command stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Command failed: {:?}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        Ok(output)
    }
}

fn format_device(ctx: &TestContext) -> io::Result<()> {
    println!("Attempting to format device: {:?}", ctx.device_path);
    let output = ctx.run_bellande_command(&["format"])?;

    if !String::from_utf8_lossy(&output.stdout).contains("Device formatted successfully") {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to format device",
        ));
    }

    Ok(())
}

fn create_and_list_files(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;

    let output = ctx.run_bellande_command(&["create", "--path", "/test.txt"])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains("File created successfully"));

    let output = ctx.run_bellande_command(&["list", "--path", "/"])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains("test.txt"));

    Ok(())
}

fn write_and_read_file(ctx: &TestContext) -> io::Result<()> {
    let test_content = "Hello, Bellande filesystem!";
    let test_file = "/test.txt";

    format_device(ctx)?;

    ctx.run_bellande_command(&["create", "--path", test_file])?;

    let mut command = Command::new(&ctx.binary_path);
    command
        .arg("--device")
        .arg(&ctx.device_path)
        .args(&["write", "--path", test_file])
        .stdin(std::process::Stdio::piped());

    let mut child = command.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(test_content.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    assert!(output.status.success());

    let output = ctx.run_bellande_command(&["read", "--path", test_file])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains(test_content));

    Ok(())
}

fn create_and_remove_directory(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;

    let output = ctx.run_bellande_command(&["mkdir", "--path", "/testdir"])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains("Directory created successfully"));

    let output = ctx.run_bellande_command(&["list", "--path", "/"])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains("testdir"));

    let output = ctx.run_bellande_command(&["rmdir", "--path", "/testdir"])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains("Directory removed successfully"));

    let output = ctx.run_bellande_command(&["list", "--path", "/"])?;
    assert!(!String::from_utf8_lossy(&output.stdout).contains("testdir"));

    Ok(())
}

fn filesystem_stats(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;

    let output = ctx.run_bellande_command(&["stats"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Total blocks"));
    assert!(stdout.contains("Free blocks"));
    assert!(stdout.contains("Total inodes"));
    assert!(stdout.contains("Free inodes"));

    Ok(())
}

fn error_handling(ctx: &TestContext) -> io::Result<()> {
    // Try to use unformatted device first
    let result = ctx.run_bellande_command(&["list", "--path", "/"]);
    assert!(result.is_err());

    format_device(ctx)?;

    let result = ctx.run_bellande_command(&["remove", "--path", "/nonexistent.txt"]);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("File not found"));
    }

    let result = ctx.run_bellande_command(&["create", "--path", "invalid/path/file.txt"]);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid path"));
    }

    Ok(())
}

fn large_file_operations(ctx: &TestContext) -> io::Result<()> {
    let large_content = "A".repeat(100_000);

    format_device(ctx)?;

    ctx.run_bellande_command(&["create", "--path", "/large.txt"])?;

    let mut command = Command::new(&ctx.binary_path);
    command
        .arg("--device")
        .arg(&ctx.device_path)
        .args(&["write", "--path", "/large.txt"])
        .stdin(std::process::Stdio::piped());

    let mut child = command.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(large_content.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    assert!(output.status.success());

    let output = ctx.run_bellande_command(&["read", "--path", "/large.txt"])?;
    assert!(String::from_utf8_lossy(&output.stdout).contains(&large_content));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executable_exists() {
        let ctx = TestContext::new().expect("Failed to create test context");
        println!("Looking for executable at: {:?}", ctx.binary_path);
        assert!(
            ctx.binary_path.exists(),
            "Executable does not exist at {:?}",
            ctx.binary_path
        );
        assert!(
            ctx.binary_path.is_file(),
            "Path {:?} is not a file",
            ctx.binary_path
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&ctx.binary_path).unwrap();
            let permissions = metadata.permissions();
            assert!(
                permissions.mode() & 0o111 != 0,
                "Executable {:?} does not have execute permissions",
                ctx.binary_path
            );
        }
    }

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

#[cfg(not(test))]
fn main() -> io::Result<()> {
    println!("Running Bellande filesystem integration tests...");
    let ctx = TestContext::new()?;
    run_full_test_suite(&ctx)
}

#[cfg(not(test))]
fn run_full_test_suite(ctx: &TestContext) -> io::Result<()> {
    format_device(ctx)?;
    create_and_list_files(ctx)?;
    write_and_read_file(ctx)?;
    create_and_remove_directory(ctx)?;
    filesystem_stats(ctx)?;
    error_handling(ctx)?;
    large_file_operations(ctx)?;
    println!("All tests passed successfully!");
    Ok(())
}
