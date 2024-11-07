use crate::utils::*;
use colored::*;

pub async fn execute(repo_name: &str) {
    print_title("🏷 ", "Creating GitHub Release", Color::Yellow);

    let version = get_cargo_toml_version().to_string();
    println!("Current Version: {}", version.to_string().bold());

    let octo = get_octocrab().await;
    let repo = octo.repos("statsig-io", repo_name);

    println!(
        "\nChecking if tag {} exists in {}...",
        version.to_string(),
        repo_name
    );

    if let Ok(_) = repo.releases().get_by_tag(&version).await {
        println!(
            "{}",
            format!("└── Release {} already exists", version).green()
        );

        return;
    }

    println!(
        "{}",
        format!("└── Release {} not found in {}", version, repo_name).yellow()
    );

    let is_prerelease =
        version.contains("-beta") || version.contains("-rc") || version.contains("-alpha");

    let branch_name = get_remote_branch_name_from_version();

    println!("-- Creating New Release --");
    println!("├── Repo: {}", repo_name);
    println!("├── Version: {}", version);
    println!("├── Branch: {}", branch_name);
    println!("└── Prerelease: {}", is_prerelease);

    match repo
        .releases()
        .create(&version)
        .target_commitish(&branch_name)
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
