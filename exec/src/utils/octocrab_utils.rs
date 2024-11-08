use colored::*;
use jsonwebtoken::EncodingKey;
use octocrab::{models::InstallationId, Octocrab, OctocrabBuilder};

pub async fn get_octocrab() -> Octocrab {
    if let Ok(app_key) = std::env::var("KONG_APP_PRIVATE_KEY") {
        println!("Authenticating with app key");

        let octo = get_octo_with_app_key(&app_key);

        if is_authenticated(&octo).await {
            let installation_id = InstallationId::from(36921303);
            let octo = octo
                .installation(installation_id)
                .expect("Failed to get installation");

            println!("Authenticated as App Installation");
            return octo;
        }
    }

    if let Ok(token) = std::env::var("OCTOCRAB_TOKEN") {
        println!("Authenticating with personal token");

        let octo = get_octo_with_personal_token(&token);
        if is_authenticated(&octo).await {
            return octo;
        }
    }

    let octo = OctocrabBuilder::new()
        .build()
        .expect("Failed to create Octocrab instance");

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

fn get_octo_with_app_key(app_key: &str) -> Octocrab {
    let key = EncodingKey::from_rsa_pem(app_key.as_bytes()).expect("Failed to parse RSA key");
    let app_id = octocrab::models::AppId(229901);

    OctocrabBuilder::new()
        .app(app_id, key)
        .build()
        .expect("Failed to create Octocrab instance")
}

fn get_octo_with_personal_token(token: &str) -> Octocrab {
    OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .expect("Failed to create Octocrab instance")
}
