use crate::utils::*;
use colored::*;

use super::sync_versions::execute as sync_versions;
use regex::Regex;

pub fn execute() {
    print_title("🏷 ", "Setting Beta Tag", Color::Yellow);

    let mut version = get_cargo_toml_version();
    println!("Current Version: {}", version.to_string().bold());

    let commit_sha = get_commit_sha();
    println!("Current Commit SHA: {}", commit_sha.bold());

    version.beta = format!("beta.{}", commit_sha);
    println!("New Version: {}", version.to_string().bold());

    write_version_to_cargo_toml(&version);
    update_grpc_dependency(&version);

    print_title(
        "✅",
        &format!("Successfully Updated Cargo.toml to {}", version.to_string()),
        Color::Green,
    );

    sync_versions()
}

fn get_commit_sha() -> String {
    let commit_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to get git commit hash")
        .stdout;

    String::from_utf8(commit_sha)
        .expect("Invalid UTF-8 in git output")
        .trim()
        .to_string()
}

fn update_grpc_dependency(version: &SemVer) {
    let cargo_toml_path = "statsig-lib/Cargo.toml";
    let content =
        std::fs::read_to_string(cargo_toml_path).expect("Failed to read statsig-lib/Cargo.toml");

    let re = Regex::new(r#"(sigstat-grpc\s*=\s*\{[^}]*version\s*=\s*")[^"]*("(?:[^}]*})?)"#)
        .expect("Failed to create regex");

    let new_content = re
        .replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}", &caps[1], version, &caps[2])
        })
        .to_string();

    std::fs::write(cargo_toml_path, new_content)
        .expect("Failed to write to statsig-lib/Cargo.toml");
}
