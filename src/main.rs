use git2::{Repository, RepositoryState};
use std::path::{Path, PathBuf};
use std::process::Command;
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
    command: &str,
    start_commit: &str,
    end_commit: &str,
) -> Result<(), String> {
    let walker = GitWalker::init(path)?;
    let range = Range {
        start: start_commit.to_owned(),
        end: end_commit.to_owned(),
    };
    let (command, args) = parse_command(command)?;
    walker.checkout_and_execute_in_range(range, || {
        let result = run_command(path, &command, &args)?;
        println!("{result}");
        Ok(true)
    })?;
    Ok(())
}

struct GitWalker {
    repo: Repository,
}

struct Range {
    start: String,
    end: String,
}

impl GitWalker {
    fn init(path: &Path) -> Result<GitWalker, String> {
        let repo = Repository::open(path).map_err(|_| "failed to open repository")?;
        if repo.state() != RepositoryState::Clean {
            return Err(String::from(
                "repository is not clean, comit any changes before running gitwalker",
            ));
        }

        Ok(GitWalker { repo })
    }

    fn checkout_and_execute_in_range(
        &self,
        range: Range,
        mut func: impl FnMut() -> Result<bool, String>,
    ) -> Result<(), String> {
        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(|err| format!("failed to create a revwalk due to {err}"))?;

        if let Err(e) = revwalk.push_range(&format!("{}..{}", range.start, range.end)) {
            return Err(format!("failed to set revwalk range: {}", e));
        };
        for commit_id in revwalk {
            let commit_id =
                commit_id.map_err(|e| format!("failed to iterate over commits due to {e}"))?;
            let commit = self
                .repo
                .find_commit(commit_id)
                .map_err(|e| format!("failed to find commit due to {e}"))?;
            self.repo
                .set_head_detached(commit.id())
                .map_err(|err| format!("failed to detach head due to {err}"))?;
            let tree = commit
                .tree()
                .map_err(|err| format!("failed to get tree due to {err}"))?;
            self.repo
                .checkout_tree(&(tree.as_object()), None)
                .map_err(|err| format!("failed to checkout commit due to {err}"))?;
            func()?;
        }
        Ok(())
    }
}

fn parse_command(command: &str) -> Result<(String, Vec<String>), String> {
    let mut parts = command.split_whitespace();
    let first = parts
        .next()
        .ok_or_else(|| String::from("empty command"))?
        .to_string();
    let rest = parts.map(|each| each.to_string()).collect::<Vec<String>>();
    Ok((first, rest))
}

fn run_command(path: &Path, command: &str, args: &[String]) -> Result<bool, String> {
    let status = Command::new(command)
        .args(args)
        .current_dir(path)
        .status()
        .map_err(|err| format!("failed to excute command {command} at {path:?} due to {err}"))?;
    Ok(status.success())
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
