use crate::utils::*;
use colored::*;

use super::beta_tag_package::execute as beta_tag_package;
use super::sync_versions::execute as sync_versions;

pub fn execute(
    major: &bool,
    minor: &bool,
    patch: &bool,
    beta: &bool,
    explicit_version: &Option<String>,
) {
    if *beta {
        beta_tag_package();
        commit_and_push_changes("./", None);
        return;
    }

    print_title("⭐️", "Bumping Version", Color::Yellow);

    let mut version = get_cargo_toml_version();

    let old_version = version.to_string();
    version.beta = "".to_string();

    if *major {
        version.major += 1;
        version.minor = 0;
        version.patch = 0;
    } else if *minor {
        version.minor += 1;
        version.patch = 0;
    } else if *patch {
        version.patch += 1;
    } else if let Some(explicit_version) = explicit_version {
        version = SemVer::from_string(explicit_version);
    } else {
        panic!("No version level specified");
    }

    write_version_to_cargo_toml(&version);

    println!(
        "Version: {} -> {}",
        old_version.bold().strikethrough(),
        version.to_string().bold()
    );

    sync_versions();
    commit_and_push_changes("./", None);
}
