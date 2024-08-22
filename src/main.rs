use git2::{Repository, RepositoryState};
use std::path::{Path, PathBuf};
use std::{env, fs};

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
        }) => execute_test(path, &command, &start, &end),
    }
}

fn execute_test(
    path: &Path,
    _command: &str,
    start_commit: &str,
    end_commit: &str,
) -> Result<(), String> {
    // TODO: given walking the commit history is common factor this to a seperate object that can
    // take actions and initialized with path, commit range
    let repo = Repository::open(path).map_err(|_| "failed to open repository")?;
    if repo.state() != RepositoryState::Clean {
        return Err(String::from(
            "repository is not clean, comit any changes before running gitwalker",
        ));
    }

    let mut revwalk = repo
        .revwalk()
        .map_err(|err| format!("failed to create a revwalk due to {err}"))?;

    if let Err(e) = revwalk.push_range(&format!("{}..{}", start_commit, end_commit)) {
        return Err(format!("failed to set revwalk range: {}", e));
    };

    for commit_id in revwalk {
        let commit_id =
            commit_id.map_err(|e| format!("failed to iterate over commits due to {e}"))?;
        let commit = repo
            .find_commit(commit_id)
            .map_err(|e| format!("failed to find commit due to {e}"))?;
        dbg!(commit_id, commit);
    }
    Ok(())
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
