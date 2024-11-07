use crate::utils::*;
use colored::*;
use git2::{Repository, Signature};

pub async fn execute() {
    print_title("⬆️ ", "Pushing to PHP Repository", Color::Yellow);

    delete_local_php_repo();

    let repo = Repository::init("statsig-php").expect("Failed to init statsig-php repo");

    repo.set_head("refs/heads/main")
        .expect("Failed to set head");

    let mut remote = repo
        .remote("origin", "https://github.com/statsig-io/sigstat-php.git")
        .expect("Failed to add remote");

    let signature =
        Signature::now("Statsig", "support@statsig.com").expect("Failed to create signature");

    let mut index = repo.index().expect("Failed to get index");
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .expect("Failed to add files to index");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "chore: update php sdk",
        &tree,
        &[],
    )
    .expect("Failed to commit changes");

    remote
        .fetch(&["refs/heads/main"], None, None)
        .expect("Failed to fetch");

    commit_and_push_changes("./statsig-php", Some("origin".to_string()));
    delete_local_php_repo();
}

fn delete_local_php_repo() {
    if !std::path::Path::new("statsig-php/.git").exists() {
        return;
    }

    std::fs::remove_dir_all("statsig-php/.git")
        .expect("Failed to delete statsig-php/.git directory");

    println!("{}", "Deleted statsig-php/.git directory".yellow());
}
