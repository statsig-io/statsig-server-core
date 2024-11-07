use crate::utils::*;
use colored::*;

pub async fn execute(commit_sha: &str, repo_name: &str) {
    print_title("🏷 ", "Creating GitHub Release", Color::Yellow);

    let version = get_cargo_toml_version().to_string();
    println!("Current Version: {}", version.to_string().bold());

    let octo = get_octocrab().await;
    let repo = octo.repos("statsig-io", repo_name);

    println!("\nChecking if tag {} exists...", version.to_string());

    if let Ok(_) = repo.releases().get_by_tag(&version).await {
        println!(
            "{}",
            format!("└── Release {} already exists", version).green()
        );

        return;
    }

    println!("{}", format!("└── Release {} not found", version).yellow());

    println!("\nCreating new release...");

    let is_prerelease =
        version.contains("-beta") || version.contains("-rc") || version.contains("-alpha");

    match repo
        .releases()
        .create(&version)
        .target_commitish(commit_sha)
        .prerelease(is_prerelease)
        .send()
        .await
    {
        Ok(_) => {
            println!("{}", "└── Release created successfully".green());
        }
        Err(e) => {
            println!("{}", "└── Failed to create release".red());
            eprintln!("\n{:#?}", e);
            panic!("Failed to create release");
        }
    };
}
