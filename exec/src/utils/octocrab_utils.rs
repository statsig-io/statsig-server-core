use colored::*;
use octocrab::{Octocrab, OctocrabBuilder};

pub async fn get_octocrab() -> Octocrab {
    let token = std::env::var("OCTOCRAB_TOKEN")
        .expect("The OCTOCRAB_TOKEN environment variable was not found");

    let octo = OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .expect("Failed to create Octocrab instance");

    if let Ok(app) = octo.current().app().await {
        println!("Successfully authenticated as: {}", app.name.bold());
    } else if let Ok(user) = octo.current().user().await {
        println!("Successfully authenticated as: {}", user.login.bold());
    } else {
        panic!("Failed to authenticate");
    }

    return octo;
}
