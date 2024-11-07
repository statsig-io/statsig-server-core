use super::get_cargo_toml_version;
use colored::*;
use git2::Repository;

pub fn get_upstream_remote_from_current_branch(working_dir: &str) -> String {
    let repo = Repository::open(working_dir).expect("Failed to open repository");
    let head = repo.head().expect("Failed to get head");

    let name = head.name().expect("Failed to get upstream");
    println!("Name: {}", name);

    let buf = repo
        .branch_upstream_remote(name)
        .expect("Failed to get upstream");

    String::from_utf8(buf.to_vec()).expect("Invalid UTF-8 in git output")
}

pub fn get_local_branch_name(working_dir: &str) -> String {
    let repo = Repository::open(working_dir).expect("Failed to open repository");
    let head = repo.head().expect("Failed to get head");

    head.shorthand()
        .expect("Failed to get branch name")
        .to_string()
}

pub fn get_remote_branch_name_from_version() -> String {
    let version = get_cargo_toml_version();
    let is_beta = version.to_string().contains("-beta");
    if is_beta {
        format!("beta/{}", version)
    } else {
        format!("release/{}", version)
    }
}

pub fn push_to_remote(working_dir: &str, remote: &str, local_branch: &str, upstream_branch: &str) {
    std::process::Command::new("git")
        .current_dir(working_dir)
        .args(&[
            "push",
            remote,
            format!("{}:{}", local_branch, upstream_branch).as_str(),
        ])
        .output()
        .expect("Failed to push to upstream");
}

pub fn commit_and_push_changes(working_dir: &str, remote_name: Option<String>) {
    std::process::Command::new("cargo")
        .args(["check"])
        .output()
        .expect("Failed to generate lockfile");

    let version = get_cargo_toml_version();
    let commit_message = format!("chore: bump version to {}", version.to_string());

    println!("Committing changes with message: {}", commit_message.bold());

    std::process::Command::new("git")
        .current_dir(working_dir)
        .args(["commit", "-am", &commit_message])
        .output()
        .expect("Failed to commit changes");

    let branch_name = get_remote_branch_name_from_version();
    println!("Pushing changes to branch: {}", branch_name.bold());

    let upstream =
        remote_name.unwrap_or_else(|| get_upstream_remote_from_current_branch(working_dir));
    let local_branch = get_local_branch_name(working_dir);

    println!("Pushing to upstream: {}", upstream.bold());
    push_to_remote(working_dir, &upstream, &local_branch, &branch_name);

    println!("Successfully pushed changes to branch");
}
