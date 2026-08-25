use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use rustyline::Editor;

struct Shell {
    cwd: PathBuf,
}

impl Shell {
    fn new() -> Self {
        Shell {
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
        }
    }

    fn run(&mut self) {
        let mut rl: Editor<(), _> = Editor::new().unwrap();

        loop {
            print!("{}> ", self.cwd.display());
            io::stdout().flush().unwrap();

            let input = match rl.readline("") {
                Ok(line) => line,
                Err(_) => break,
            };

            if input.trim().is_empty() {
                continue;
            }

            rl.add_history_entry(&input).ok();

            if let Err(e) = self.execute(&input) {
                eprintln!("Error: {}", e);
            }
        }
    }

    fn execute(&mut self, input: &str) -> Result<(), String> {
        let tokens = self.parse(input);
        self.run_pipeline(&tokens)
    }

    fn parse(&self, input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        let mut quote_char = ' ';

        for c in input.chars() {
            if in_quote {
                if c == quote_char {
                    in_quote = false;
                } else {
                    current.push(c);
                }
            } else if c == '"' || c == '\'' {
                in_quote = true;
                quote_char = c;
            } else if c == ' ' || c == '\t' {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(c);
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    fn run_pipeline(&mut self, tokens: &[String]) -> Result<(), String> {
        let mut commands: Vec<Vec<String>> = Vec::new();
        let mut current_cmd: Vec<String> = Vec::new();

        for token in tokens {
            if token == "|" {
                if current_cmd.is_empty() {
                    return Err("Invalid pipe syntax".to_string());
                }
                commands.push(current_cmd.clone());
                current_cmd.clear();
            } else {
                current_cmd.push(token.clone());
            }
        }

        if !current_cmd.is_empty() {
            commands.push(current_cmd);
        }

        if commands.is_empty() {
            return Ok(());
        }

        if commands.len() == 1 {
            self.run_single_command(&commands[0])?;
        } else {
            self.run_piped_commands(&commands)?;
        }

        Ok(())
    }

    fn run_single_command(&mut self, args: &[String]) -> Result<(), String> {
        if args.is_empty() {
            return Ok(());
        }

        let (cmd, args, redirects) = self.extract_redirects(args.to_vec());

        if let Some((input_file, output_file, append)) = redirects {
            self.run_command_with_redirects(&cmd, &args, input_file, output_file, append)?;
        } else {
            self.run_command(&cmd, &args)?;
        }

        Ok(())
    }

    fn extract_redirects(&self, args: Vec<String>) -> (String, Vec<String>, Option<(Option<String>, Option<String>, bool)>) {
        let mut input_file = None;
        let mut output_file = None;
        let mut append = false;
        let mut new_args = Vec::new();
        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "<" => {
                    if i + 1 < args.len() {
                        input_file = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        new_args.push(args[i].clone());
                        i += 1;
                    }
                }
                ">" => {
                    append = false;
                    if i + 1 < args.len() {
                        output_file = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        new_args.push(args[i].clone());
                        i += 1;
                    }
                }
                ">>" => {
                    append = true;
                    if i + 1 < args.len() {
                        output_file = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        new_args.push(args[i].clone());
                        i += 1;
                    }
                }
                _ => {
                    new_args.push(args[i].clone());
                    i += 1;
                }
            }
        }

        let cmd = if new_args.is_empty() {
            String::new()
        } else {
            new_args[0].clone()
        };

        let args = if new_args.len() > 1 {
            new_args[1..].to_vec()
        } else {
            Vec::new()
        };

        let redirects = if input_file.is_some() || output_file.is_some() {
            Some((input_file, output_file, append))
        } else {
            None
        };

        (cmd, args, redirects)
    }

    fn run_command_with_redirects(
        &self,
        cmd: &str,
        args: &[String],
        input_file: Option<String>,
        output_file: Option<String>,
        append: bool,
    ) -> Result<(), String> {
        let mut child = Command::new(cmd);
        child.args(args);

        if let Some(ref input) = input_file {
            let file = File::open(input).map_err(|e| format!("Failed to open {}: {}", input, e))?;
            #[cfg(unix)]
            use std::os::unix::io::{FromRawFd, IntoRawFd};
            #[cfg(windows)]
            use std::os::windows::io::{FromRawHandle, IntoRawHandle};

            #[cfg(unix)]
            let stdin = unsafe { Stdio::from_raw_fd(file.into_raw_fd()) };
            #[cfg(windows)]
            let stdin = unsafe { Stdio::from_raw_handle(file.into_raw_handle()) };
            child.stdin(stdin);
        }

        if let Some(ref output) = output_file {
            let file = if append {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(output)
                    .map_err(|e| format!("Failed to open {}: {}", output, e))?
            } else {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(output)
                    .map_err(|e| format!("Failed to open {}: {}", output, e))?
            };
            #[cfg(unix)]
            use std::os::unix::io::{FromRawFd, IntoRawFd};
            #[cfg(windows)]
            use std::os::windows::io::{FromRawHandle, IntoRawHandle};

            #[cfg(unix)]
            let stdout = unsafe { Stdio::from_raw_fd(file.into_raw_fd()) };
            #[cfg(windows)]
            let stdout = unsafe { Stdio::from_raw_handle(file.into_raw_handle()) };
            child.stdout(stdout);
        }

        child.spawn().map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;
        Ok(())
    }

    fn run_piped_commands(&self, commands: &[Vec<String>]) -> Result<(), String> {
        if commands.is_empty() {
            return Ok(());
        }

        let mut processes: Vec<process::Child> = Vec::new();
        let mut prev_stdout: Option<Stdio> = None;

        for (i, cmd_args) in commands.iter().enumerate() {
            let (cmd, args, _) = self.extract_redirects(cmd_args.clone());

            let mut child = Command::new(&cmd);
            child.args(&args);

            if let Some(stdout) = prev_stdout.take() {
                child.stdin(stdout);
            }

            if i < commands.len() - 1 {
                let mut proc = child.stdout(Stdio::piped()).spawn()
                    .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;
                let out = proc.stdout.take().unwrap();
                prev_stdout = Some(Stdio::from(out));
                processes.push(proc);
            } else {
                child.spawn().map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;
            }
        }

        for mut child in processes {
            child.wait().ok();
        }

        Ok(())
    }

    fn run_command(&mut self, cmd: &str, args: &[String]) -> Result<(), String> {
        match cmd {
            "cd" => self.cmd_cd(args),
            "pwd" => self.cmd_pwd(),
            "echo" => self.cmd_echo(args),
            "exit" => self.cmd_exit(args),
            "env" => self.cmd_env(args),
            "set" => self.cmd_set(args),
            "export" => self.cmd_export(args),
            "unset" => self.cmd_unset(args),
            "history" => Ok(()),
            _ => self.run_external_command(cmd, args),
        }
    }

    fn cmd_cd(&mut self, args: &[String]) -> Result<(), String> {
        let dir = if args.is_empty() {
            env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        PathBuf::from("C:\\")
                    } else {
                        PathBuf::from("/")
                    }
                })
        } else {
            PathBuf::from(&args[0])
        };

        let target = if dir.is_absolute() {
            dir
        } else {
            self.cwd.join(dir)
        };

        if !target.exists() {
            return Err(format!("cd: {}: No such file or directory", target.display()));
        }

        if !target.is_dir() {
            return Err(format!("cd: {}: Not a directory", target.display()));
        }

        self.cwd = target.canonicalize()
            .map_err(|e| format!("cd: {}", e))?;
        env::set_current_dir(&self.cwd).map_err(|e| format!("cd: {}", e))?;
        Ok(())
    }

    fn cmd_pwd(&self) -> Result<(), String> {
        println!("{}", self.cwd.display());
        Ok(())
    }

    fn cmd_echo(&self, args: &[String]) -> Result<(), String> {
        println!("{}", args.join(" "));
        Ok(())
    }

    fn cmd_exit(&self, args: &[String]) -> Result<(), String> {
        let code = args.first()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        std::process::exit(code);
    }

    fn cmd_env(&self, args: &[String]) -> Result<(), String> {
        if args.is_empty() {
            for (key, value) in env::vars() {
                println!("{}={}", key, value);
            }
        } else {
            for arg in args {
                if let Ok(value) = env::var(arg) {
                    println!("{}={}", arg, value);
                }
            }
        }
        Ok(())
    }

    fn cmd_set(&self, args: &[String]) -> Result<(), String> {
        if args.len() < 2 {
            return Err("set: expected NAME VALUE".to_string());
        }
        env::set_var(&args[0], &args[1]);
        Ok(())
    }

    fn cmd_export(&self, args: &[String]) -> Result<(), String> {
        if args.is_empty() {
            for (key, value) in env::vars() {
                println!("export {}={}", key, value);
            }
            return Ok(());
        }
        for arg in args {
            if let Some((key, value)) = arg.split_once('=') {
                env::set_var(key, value);
            } else {
                env::remove_var(arg);
            }
        }
        Ok(())
    }

    fn cmd_unset(&self, args: &[String]) -> Result<(), String> {
        for arg in args {
            env::remove_var(arg);
        }
        Ok(())
    }

    fn run_external_command(&self, cmd: &str, args: &[String]) -> Result<(), String> {
        #[cfg(windows)]
        {
            let shell_aliases = ["powershell", "pwsh", "cmd", "cmd.exe", "powershell.exe", "pwsh.exe"];
            if shell_aliases.iter().any(|&s| cmd.eq_ignore_ascii_case(s)) {
                if cmd.eq_ignore_ascii_case("pwsh") || cmd.eq_ignore_ascii_case("pwsh.exe") {
                    let mut child = Command::new("C:\\Program Files\\PowerShell\\7\\pwsh.exe");
                    child.stdin(Stdio::inherit());
                    child.stdout(Stdio::inherit());
                    child.stderr(Stdio::inherit());

                    let status = child
                        .status()
                        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

                    if !status.success() {
                        return Err(format!("Command {} exited with code {:?}", cmd, status.code()));
                    }
                    return Ok(());
                } else if cmd.eq_ignore_ascii_case("powershell") || cmd.eq_ignore_ascii_case("powershell.exe") {
                    let mut child = Command::new("cmd");
                    child.args(&["/c", "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"]);
                    child.stdin(Stdio::inherit());
                    child.stdout(Stdio::inherit());
                    child.stderr(Stdio::inherit());
                    child.creation_flags(0x08000000);

                    let status = child
                        .status()
                        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

                    if !status.success() {
                        return Err(format!("Command {} exited with code {:?}", cmd, status.code()));
                    }
                    return Ok(());
                } else {
                    let mut child = Command::new("cmd");
                    child.args(args);
                    child.stdin(Stdio::inherit());
                    child.stdout(Stdio::inherit());
                    child.stderr(Stdio::inherit());
                    child.creation_flags(0x08000000);

                    let status = child
                        .status()
                        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

                    if !status.success() {
                        return Err(format!("Command {} exited with code {:?}", cmd, status.code()));
                    }
                    return Ok(());
                }
            }
        }

        let status = Command::new(cmd)
            .args(args)
            .status()
            .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

        if !status.success() {
            return Err(format!("Command {} exited with code {:?}", cmd, status.code()));
        }

        Ok(())
    }
}

fn main() {
    #[cfg(windows)]
    {
        let mut child = Command::new("C:\\Program Files\\PowerShell\\7\\pwsh.exe");
        child.stdin(Stdio::inherit());
        child.stdout(Stdio::inherit());
        child.stderr(Stdio::inherit());
        std::process::exit(match child.status() {
            Ok(status) => status.code().unwrap_or(1) as i32,
            Err(_) => 1,
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