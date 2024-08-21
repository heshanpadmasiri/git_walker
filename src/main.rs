use git2::{Repository, RepositoryState};

fn main() {
    // FIXME: can't use "~/Projects/ballerina-lang" here
    let TEST_REPO = "/Users/heshanpadmasiri/Projects/ballerina-lang";
    let repo = match Repository::open(TEST_REPO) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open {}", e)
    };
    if repo.state() != RepositoryState::Clean {
        panic!("Repository state is not clean");
    }
    let start_commit = "e27ccd56";
    let end_commit = "8f9b0527";

    let mut revwalk = match repo.revwalk() {
        Ok(revwalk) => revwalk,
        Err(e) => panic!("failed to create revwalk: {}", e),
    };

    if let Err(e) = revwalk.push_range(&format!("{}..{}", start_commit, end_commit)) {
        panic!("failed to set revwalk range: {}", e);
    }

    for commit_id in revwalk {
        match commit_id {
            Ok(commit_id) => {
                let commit = match repo.find_commit(commit_id) {
                    Ok(commit) => commit,
                    Err(e) => panic!("failed to find commit: {}", e),
                };
                // TODO: checkout commit
                // TODO: run the given command at the commit and check the status code
                println!("{}", commit.id());
            }
            Err(e) => panic!("failed to iterate over commits: {}", e),
        }
    }
}
