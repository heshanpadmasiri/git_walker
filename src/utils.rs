use git2::{Commit, ObjectType, Oid, Repository, RepositoryState};
use std::path::Path;

#[derive(Debug)]
pub struct Command {
    pub command: String,
    pub args: Vec<String>,
    pub verbose: bool,
}

impl Command {
    pub fn as_verbose(&self) -> Command {
        Command {
            command: self.command.clone(),
            args: self.args.clone(),
            verbose: true,
        }
    }
}

impl TryFrom<&String> for Command {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        let (command, args) = parse_command(value)?;
        Ok(Command {
            command,
            args,
            verbose: false,
        })
    }
}

impl TryFrom<String> for Command {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (command, args) = parse_command(&value)?;
        Ok(Command {
            command,
            args,
            verbose: false,
        })
    }
}

impl TryFrom<&str> for Command {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (command, args) = parse_command(value)?;
        Ok(Command {
            command,
            args,
            verbose: false,
        })
    }
}

#[derive(Debug)]
struct AtomizeCandidate {
    failing_commits: Vec<Oid>,
    target_commit: Oid,
}

#[derive(Debug)]
struct AtomizationContext {
    candidates: Vec<AtomizeCandidate>,
    last_good: Option<(Oid, Vec<Oid>)>,
}

enum CommitState {
    Working,
    Failing,
}

impl AtomizationContext {
    fn new() -> AtomizationContext {
        return AtomizationContext {
            candidates: vec![],
            last_good: None,
        };
    }

    fn append_commit(&mut self, state: CommitState, id: Oid) {
        match state {
            CommitState::Working => {
                if let Some((target_commit, failing_commits)) = &self.last_good {
                    self.candidates.push(AtomizeCandidate {
                        failing_commits: failing_commits.clone(),
                        target_commit: *target_commit,
                    });
                }
                self.last_good = Some((id, vec![]));
            }
            CommitState::Failing => {
                if let Some((_, failing_commits)) = &mut self.last_good {
                    failing_commits.push(id);
                }
            }
        }
    }
}

pub fn execute_atomize(
    path: &Path,
    command: &Command,
    start_commit: &str,
    end_commit: &str,
) -> Result<(), String> {
    let walker = GitWalker::init(path)?;
    let range = Range {
        start: start_commit.to_owned(),
        end: end_commit.to_owned(),
    };
    let mut context = AtomizationContext::new();
    let _ = walker.checkout_and_execute_in_range(range, |commit| {
        let state = if run_command(path, &command.command, &command.args, !command.verbose)? {
            CommitState::Working
        } else {
            CommitState::Failing
        };
        context.append_commit(state, commit.id());
        Ok(true)
    });
    dbg!(context);
    todo!()
}

pub fn execute_test(
    path: &Path,
    command: &Command,
    start_commit: &str,
    end_commit: &str,
) -> Result<(), String> {
    let walker = GitWalker::init(path)?;
    let range = Range {
        start: start_commit.to_owned(),
        end: end_commit.to_owned(),
    };
    walker.checkout_and_execute_in_range(range, |commit| {
        let result = run_command(path, &command.command, &command.args, !command.verbose)?;
        let result = if result { "✓" } else { "✗" };
        let commit_id = commit.id();
        match commit_message(commit) {
            Some(message) => println!("{commit_id} : {message} : {result}"),
            None => println!("{commit_id} : {result}"),
        }
        Ok(true)
    })?;
    Ok(())
}

fn commit_message(commit: &Commit) -> Option<String> {
    match commit.message() {
        Some(message) => {
            let message = message.trim();
            if message.is_empty() {
                None
            } else {
                let len = message.len();
                if len < 20 {
                    Some(format!("{:<23}", message))
                } else {
                    Some(format!("{:.20}...", message))
                }
            }
        }
        None => None,
    }
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
        let repo = Repository::open(path)
            .map_err(|err| format!("failed to open repository due to {err}"))?;
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
        func: impl FnMut(&Commit) -> Result<bool, String>,
    ) -> Result<(), String> {
        let current_branch = self.get_current_branch();
        let result = self.checkout_and_execute_in_range_inner(range, func);
        if let Some(branch) = current_branch {
            self.checkout_branch(&branch)?;
        }
        result
    }

    fn get_current_branch(&self) -> Option<String> {
        self.repo
            .head()
            .ok()
            .and_then(|head| head.shorthand().map(|s| s.to_string()))
    }

    fn checkout_branch(&self, branch: &str) -> Result<(), String> {
        let obj = self
            .repo
            .revparse_single(&format!("refs/heads/{}", branch))
            .map_err(|err| format!("failed to find the branch due to {err}"))?;
        self.repo
            .checkout_tree(&obj, None)
            .map_err(|err| format!("failed to checkout branch {branch} due to {err}"))?;
        self.repo
            .set_head(&format!("refs/heads/{}", branch))
            .map_err(|err| format!("failed to set head to {branch} due to {err}"))?;
        Ok(())
    }

    fn checkout_and_execute_in_range_inner(
        &self,
        range: Range,
        mut func: impl FnMut(&Commit) -> Result<bool, String>,
    ) -> Result<(), String> {
        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(|err| format!("failed to create a revwalk due to {err}"))?;

        let revspec = self
            .repo
            .revparse(&format!("{}..{}", range.start, range.end))
            .unwrap();
        let from = revspec.from().unwrap().id();
        let to = revspec.to().unwrap().id();
        revwalk.push(to).unwrap();
        if revspec.mode().contains(git2::RevparseMode::MERGE_BASE) {
            let base = self.repo.merge_base(from, to).unwrap();
            let o = self
                .repo
                .find_object(base, Some(ObjectType::Commit))
                .unwrap();
            revwalk.push(o.id()).unwrap();
        }
        revwalk.hide(from).unwrap();
        let commit_ids = revwalk
            .collect::<Result<Vec<Oid>, git2::Error>>()
            .map_err(|e| format!("failed to collect commit ids due to {e}"))?;
        for commit_id in commit_ids {
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
                .checkout_tree(tree.as_object(), None)
                .map_err(|err| format!("failed to checkout commit due to {err}"))?;
            self.repo
                .reset(commit.as_object(), git2::ResetType::Hard, None)
                .map_err(|err| format!("failed to reset to commit due to {err}"))?;
            func(&commit)?;
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

fn run_command(path: &Path, command: &str, args: &[String], silent: bool) -> Result<bool, String> {
    let status = std::process::Command::new(command)
        .args(args)
        .current_dir(path)
        .stdout(if silent {
            std::process::Stdio::null()
        } else {
            std::process::Stdio::inherit()
        })
        .status()
        .map_err(|err| format!("failed to excute command {command} at {path:?} due to {err}"))?;

    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    use tempfile::TempDir;

    fn create_temp_copy_of_test_repository() -> TempDir {
        let temp_dir = TempDir::new().expect("failed to create temp directory");
        let submodule_path = PathBuf::from(std::env::var("TEST_REPO").unwrap());
        let target_path = temp_dir.path().join("gitwalker_test_repo");

        copy_directory(&submodule_path, &target_path);

        temp_dir
    }

    fn copy_directory(source: &Path, target: &Path) {
        if !target.exists() {
            fs::create_dir_all(target).expect("failed to create target directory");
        }

        for entry in fs::read_dir(source).expect("failed to read source directory") {
            let entry = entry.expect("failed to get entry");
            let source_path = entry.path();
            let target_path = target.join(entry.file_name());
            if source_path.is_dir() {
                copy_directory(&source_path, &target_path);
            } else {
                fs::copy(&source_path, &target_path).expect("failed to copy file");
            }
        }
    }

    #[test]
    fn detect_empty_command() {
        let command = "";
        let result = parse_command(command);
        assert!(result.is_err());
    }

    #[test]
    fn detect_command_with_no_args() {
        let command = "python3";
        let result = parse_command(command);
        assert!(result.is_ok());
        let (first, rest) = result.unwrap();
        assert_eq!(first, "python3");
        assert_eq!(rest.len(), 0);
    }

    #[test]
    fn correctly_parse_command_with_args() {
        let command = "python3 test.py";
        let result = parse_command(command);
        assert!(result.is_ok());
        let (first, rest) = result.unwrap();
        assert_eq!(first, "python3");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], "test.py");
    }

    #[test]
    fn test_walker_checkout_and_execute() {
        let temp_dir = create_temp_copy_of_test_repository();
        let walker = GitWalker::init(&temp_dir.path().join("gitwalker_test_repo")).unwrap();
        let range = Range {
            start: String::from("470ba9d"),
            end: String::from("59d58e3"),
        };
        let (command, args) = parse_command("python3 test.py").unwrap();
        let expected_results = vec![true, false];
        let mut index = 0;
        walker
            .checkout_and_execute_in_range(range, |commit| {
                let result = run_command(
                    &temp_dir.path().join("gitwalker_test_repo"),
                    &command,
                    &args,
                    true,
                )
                .unwrap();
                dbg!(result, index, commit);
                let expected_result = expected_results.get(index).unwrap();
                index += 1;
                assert_eq!(result, *expected_result);
                Ok(true)
            })
            .unwrap();
        assert_eq!(index, expected_results.len());
    }
}
