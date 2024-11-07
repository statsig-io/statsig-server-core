use std::path::Path;

use crate::utils::*;
use bytes::Bytes;
use colored::*;
use octocrab::{models::repos::Release, repos::RepoHandler};

pub async fn execute(asset_path: &str, repo_name: &str) {
    print_title("🏷 ", "Attaching Asset to GitHub Release", Color::Yellow);

    let octo = get_octocrab().await;
    let repo = octo.repos("statsig-io", repo_name);

    let release = get_release_by_version(&repo).await;
    let file_bytes = get_asset_bytes(asset_path);
    let file_name = Path::new(asset_path)
        .file_name()
        .and_then(|f| f.to_str())
        .expect("Invalid asset path");

    upload_asset(&repo, release, file_bytes, file_name).await;
}

async fn get_release_by_version(repo: &RepoHandler<'_>) -> Release {
    let version = get_cargo_toml_version();
    println!("Current Version: {}", version.to_string().bold());

    let release = repo
        .releases()
        .get_by_tag(&version.to_string())
        .await
        .expect("Unable to find release");

    println!("Found release: {}", release.id.into_inner());

    release
}

fn get_asset_bytes(asset_path: &str) -> Bytes {
    println!("Reading asset file...");

    let file_data = std::fs::read(asset_path).expect("Failed to read asset file");
    let file_bytes: Bytes = Bytes::from(file_data);

    println!(
        "Asset file read successfully. Byte size: {}",
        file_bytes.len()
    );

    file_bytes
}

async fn upload_asset(
    repo: &RepoHandler<'_>,
    release: Release,
    file_bytes: Bytes,
    file_name: &str,
) {
    let assets = repo
        .releases()
        .assets(release.id.into_inner())
        .per_page(100)
        .send()
        .await
        .expect("Failed to get release assets");

    if let Some(asset) = assets.items.iter().find(|x| x.name.eq(file_name)) {
        println!("Asset {} already exists. Deleting...", file_name);

        repo.release_assets()
            .delete(asset.id.into_inner())
            .await
            .expect("Failed to delete asset");
        println!(
            "{}",
            format!("└── Asset {} deleted successfully.", file_name).green()
        );
    } else {
        println!(
            "{}",
            format!("└── Asset {} does not yet exist.", file_name).yellow()
        );
    }

    println!("Attaching asset {}...", file_name);

    match repo
        .releases()
        .upload_asset(release.id.into_inner(), file_name, file_bytes)
        .send()
        .await
    {
        Ok(_) => {
            println!("{}", "└── Asset attached successfully".green());
        }
        Err(e) => {
            println!("{}", "└── Failed to attach asset".red());
            eprintln!("\n{:#?}", e);
            panic!("Failed to attach asset");
        }
    };
}
