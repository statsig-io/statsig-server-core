use git2::Repository;

use super::get_cargo_toml_version;

pub fn get_upstream_branch() -> String {
    let repo = Repository::open(".").expect("Failed to open repository");
    let head = repo.head().expect("Failed to get head");

    let name = head.name().expect("Failed to get upstream");
    println!("Name: {}", name);

    let buf = repo
        .branch_upstream_remote(name)
        .expect("Failed to get upstream");

    String::from_utf8(buf.to_vec()).expect("Invalid UTF-8 in git output")
}

pub fn get_local_branch_name() -> String {
    let repo = Repository::open(".").expect("Failed to open repository");
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

pub fn push_to_remote(remote: &str, local_branch: &str, upstream_branch: &str) {
    std::process::Command::new("git")
        .args(&[
            "push",
            remote,
            format!("{}:{}", local_branch, upstream_branch).as_str(),
        ])
        .output()
        .expect("Failed to push to upstream");
}
