use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod utils;
#[derive(Debug)]
struct TestCommand {
    start: String,
    end: String,
    command: String,
}

#[derive(Debug)]
enum Action {
    Test(TestCommand),
}
#[derive(Debug)]
struct Args {
    path: PathBuf,
    action: Action,
}

fn main() -> Result<(), String> {
    match parse_args() {
        Ok(Args { path, action }) => execute_action(&path, action),
        Err(err) => Err(format!("failed to parse arguments due to {err}")),
    }
}

fn execute_action(path: &Path, action: Action) -> Result<(), String> {
    match action {
        Action::Test(TestCommand {
            start,
            end,
            command,
        }) => utils::execute_test(path, &command, &start, &end),
    }
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();

    match args.as_slice() {
        [_, test, path, start_commit, end_commit, remaining_args @ ..] if test == "test" => {
            let path = validate_and_get_absolute_path(path)?;
            finish_parsing_test_args(remaining_args, path, start_commit, end_commit)
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
    match remaining_args {
        [] => Err(String::from("expected a command to test")),
        [command] => Ok(Args {
            path,
            action: Action::Test(TestCommand {
                start: start_commit.to_string(),
                end: end_commit.to_string(),
                command: command.to_string(),
            }),
        }),
        _ => Err(String::from("too many arguments")),
    }
}
