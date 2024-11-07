mod commands;
mod utils;

use clap::{Parser, Subcommand};
use commands::*;

#[derive(Parser)]
#[command(name = "exec")]
#[command(about = "Workspace build tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    BumpVersion {
        #[arg(long, required = false)]
        major: bool,

        #[arg(long, required = false)]
        minor: bool,

        #[arg(long, required = false)]
        patch: bool,

        #[arg(long, required = false)]
        beta: bool,

        #[arg(long, required = false)]
        version: Option<String>,
    },
    BetaTagPackage,
    SyncVersions,
    BuildNode {
        #[arg(long, required = false)]
        target: Option<String>,

        #[arg(long, required = false)]
        release: bool,

        #[arg(long, required = false)]
        use_napi_cross: bool,

        #[arg(long, required = false)]
        cross_compile: bool,

        #[arg(long, required = false)]
        vendored_openssl: bool,
    },
    CreateGhRelease {
        #[arg(long, required = true)]
        repo_name: String,
    },
    AttachGhAssets {
        #[arg(long, required = true)]
        asset_path: String,

        #[arg(long, required = true)]
        repo_name: String,
    },
    ZipFiles {
        #[arg(long, required = true)]
        pattern: String,

        #[arg(long, required = true)]
        output: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::BumpVersion {
            major,
            minor,
            patch,
            beta,
            version,
        } => bump_version::execute(major, minor, patch, beta, version),
        Commands::BetaTagPackage => beta_tag_package::execute(),
        Commands::SyncVersions => sync_versions::execute(),
        Commands::BuildNode {
            target,
            release,
            use_napi_cross,
            cross_compile,
            vendored_openssl,
        } => build_node::execute(
            target,
            release,
            use_napi_cross,
            cross_compile,
            vendored_openssl,
        ),
        Commands::CreateGhRelease { repo_name } => create_gh_release::execute(repo_name).await,
        Commands::AttachGhAssets {
            asset_path,
            repo_name,
        } => attach_gh_assets::execute(asset_path, repo_name).await,
        Commands::ZipFiles { pattern, output } => zip_files::execute(pattern, output).await,
    }
}
