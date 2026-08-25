use std::process::{Command, Stdio};

#[cfg(not(windows))]
use std::env;

fn main() {
    #[cfg(windows)]
    {
        let shell = if which::which("pwsh").is_ok() {
            "pwsh"
        } else if which::which("powershell").is_ok() {
            "powershell"
        } else {
            "cmd"
        };

        let mut child = Command::new(shell);
        child.stdin(Stdio::inherit());
        child.stdout(Stdio::inherit());
        child.stderr(Stdio::inherit());
        std::process::exit(match child.status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => {
                eprintln!("Failed to start {}: {}", shell, e);
                1
            }
        });
    }

    #[cfg(not(windows))]
    {
        let shell_path = env::var("SHELL")
            .unwrap_or_else(|_| "/bin/bash".to_string());
        let shell_name = std::path::Path::new(&shell_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bash");

        let mut child = Command::new(&shell_path);
        child.stdin(Stdio::inherit());
        child.stdout(Stdio::inherit());
        child.stderr(Stdio::inherit());

        std::process::exit(match child.status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => {
                eprintln!("Failed to start {}: {}", shell_name, e);
                1
            }
        });
    }
}
