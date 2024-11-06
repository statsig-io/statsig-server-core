use colored::*;
use octocrab::{Octocrab, OctocrabBuilder};

pub async fn get_octocrab() -> Octocrab {
    let token = std::env::var("OCTOCRAB_TOKEN")
        .expect("The OCTOCRAB_TOKEN environment variable was not found");

    let octo = OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .expect("Failed to create Octocrab instance");

    match octo.current().user().await {
        Ok(user) => {
            println!("Successfully authenticated as: {}", user.login.bold());
            user
        }
        Err(e) => {
            eprintln!("Authentication error: {:#?}", e);
            panic!("Failed to authenticate");
        }
    };

    return octo;
}
