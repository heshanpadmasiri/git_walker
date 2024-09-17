use crate::utils::Command;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod utils;

#[derive(Debug)]
struct TestCommand {
    path: PathBuf,
    start: String,
    end: String,
    command: Command,
}

#[derive(Debug)]
enum Action {
    Test(TestCommand),
    Help,
}
#[derive(Debug)]
struct Args {
    action: Action,
}

fn main() -> Result<(), String> {
    match parse_args() {
        Ok(Args { action }) => execute_action(action),
        Err(err) => Err(format!("failed to parse arguments due to {err}")),
    }
}

fn execute_action(action: Action) -> Result<(), String> {
    match action {
        Action::Test(TestCommand {
            start,
            end,
            command,
            path,
        }) => utils::execute_test(&path, &command, &start, &end),
        Action::Help => Ok(print_help()),
    }
}

fn print_help() {
    println!("Usage: git-walker <test> <path> <start-commit> <end-commit> <command> [--verbose]");
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();
    let command = args.iter().nth(1).ok_or("expected a command")?;
    let args = args
        .iter()
        .skip(2)
        .map(|arg| arg.to_string())
        .collect::<Vec<String>>();
    if command == "--help" || command == "-h" {
        return Ok(Args {
            action: Action::Help,
        });
    }
    match args.as_slice() {
        [path, start_commit, end_commit, remaining_args @ ..] => {
            let path = validate_and_get_absolute_path(path)?;
            match command.as_str() {
                "test" => finish_parsing_test_args(remaining_args, path, start_commit, end_commit),
                _ => Err(String::from("invalid command")),
            }
        }
        _ => Err(String::from("invalid comand")),
    }
}

fn validate_and_get_absolute_path(path_str: &str) -> Result<PathBuf, String> {
    let path = Path::new(path_str);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let current_dir = env::current_dir()
            .map_err(|err| format!("failed to get current directory due to {err}"))?;
        current_dir.join(path)
    };
    fs::canonicalize(path).map_err(|err| format!("invalid path {err}"))
}

fn finish_parsing_test_args(
    remaining_args: &[String],
    path: PathBuf,
    start_commit: &str,
    end_commit: &str,
) -> Result<Args, String> {
    let command = parse_command(remaining_args)?;
    Ok(Args {
        action: Action::Test(TestCommand {
            start: start_commit.to_string(),
            end: end_commit.to_string(),
            command,
            path,
        }),
    })
}

fn parse_command(remaining_args: &[String]) -> Result<Command, String> {
    match remaining_args {
        [] => Err(String::from("expected a command")),
        [command] => Command::try_from(command),
        [command, rest @ ..] => {
            let mut command = Command::try_from(command)?;
            let options = parse_options(rest)?;
            if options.verbose {
                command = command.as_verbose();
            }
            Ok(command)
        }
    }
}

struct ArgOptions {
    verbose: bool,
}

impl Default for ArgOptions {
    fn default() -> Self {
        Self { verbose: false }
    }
}

fn parse_options(remaining_args: &[String]) -> Result<ArgOptions, String> {
    let mut options = ArgOptions::default();
    for arg in remaining_args {
        match arg.as_str() {
            "--verbose" => options.verbose = true,
            _ => return Err(format!("unknown option {arg}")),
        }
    }
    Ok(options)
}
