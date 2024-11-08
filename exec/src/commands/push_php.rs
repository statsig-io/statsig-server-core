use crate::utils::*;
use colored::*;
use git2::Repository;

pub async fn execute() {
    print_title("⬆️ ", "Pushing to PHP Repository", Color::Yellow);

    delete_local_php_repo();

    let repo = Repository::init("statsig-php").expect("Failed to init statsig-php repo");

    repo.set_head("refs/heads/main")
        .expect("Failed to set head");

    let remote = repo
        .remote("origin", "https://github.com/statsig-io/sigstat-php.git")
        .expect("Failed to add remote");

    println!(
        "Added remote origin to statsig-php: {}",
        remote.url().expect("Failed to get remote url")
    );

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
