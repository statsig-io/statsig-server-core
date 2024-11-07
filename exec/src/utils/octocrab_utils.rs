use colored::*;
use octocrab::{Octocrab, OctocrabBuilder};

pub async fn get_octocrab() -> Octocrab {
    let token = std::env::var("OCTOCRAB_TOKEN")
        .expect("The OCTOCRAB_TOKEN environment variable was not found");

    let octo = OctocrabBuilder::new()
        .personal_token(token.clone())
        .build()
        .expect("Failed to create Octocrab instance");

    if is_authenticated(&octo).await {
        return octo;
    }

    println!("Unknown authentication. Things may not work as expected.");
    return octo;
}

async fn is_authenticated(octo: &Octocrab) -> bool {
    if let Ok(app) = octo.current().app().await {
        println!("Successfully authenticated as: {}", app.name.bold());
        true
    } else if let Ok(user) = octo.current().user().await {
        println!("Successfully authenticated as: {}", user.login.bold());
        true
    } else {
        false
    }
}
