use std::collections::HashMap;

use crate::utils::{get_cargo_toml_version, print_title};
use colored::{Color, Colorize};
use config::{self, Config, File, FileFormat};
use serde_json::json;

pub fn execute() {
    print_title("🔄", "Syncing Versions", Color::Yellow);

    let version = get_cargo_toml_version();
    println!("Current Version: {}", version.to_string().bold());

    let node_version = get_node_package_json_version();
    set_node_package_json_version(version.to_string());
    println!(
        "Node Version: {} -> {}",
        node_version.bold().strikethrough(),
        version.to_string().bold()
    );

    let java_version = get_java_gradle_version();
    set_java_gradle_version(version.to_string());
    println!(
        "Java Version: {} -> {}",
        java_version.bold().strikethrough(),
        version.to_string().bold()
    );

    print_title(
        "✅",
        &format!("All Versions Updated to: {}", version.to_string()),
        Color::Green,
    );
}

fn get_node_package_json_version() -> String {
    let file =
        std::fs::read_to_string("statsig-napi/package.json").expect("Failed to read package.json");

    let json: serde_json::Value =
        serde_json::from_str(&file).expect("Failed to parse package.json");

    json["version"].as_str().unwrap().to_string()
}

fn set_node_package_json_version(version: String) {
    let file =
        std::fs::read_to_string("statsig-napi/package.json").expect("Failed to read package.json");

    let mut json: serde_json::Value =
        serde_json::from_str(&file).expect("Failed to parse package.json");

    json["version"] = json!(version);

    std::fs::write(
        "statsig-napi/package.json",
        serde_json::to_string_pretty(&json).expect("Failed to format JSON"),
    )
    .expect("Failed to write to package.json");
}

fn get_java_properties() -> HashMap<String, String> {
    let properties = Config::builder()
        .add_source(File::new(
            "statsig-ffi/bindings/java/gradle.properties",
            FileFormat::Ini,
        ))
        .build()
        .expect("Failed to build gradle.properties");

    properties
        .try_deserialize::<HashMap<String, String>>()
        .expect("Failed to deserialize gradle.properties")
}

fn get_java_gradle_version() -> String {
    let map = get_java_properties();
    map["version"].clone()
}

fn set_java_gradle_version(version: String) {
    let mut map = get_java_properties();
    map.insert("version".to_string(), version);

    // Write the updated properties back to the file
    let content = map
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join("\n");

    std::fs::write("statsig-ffi/bindings/java/gradle.properties", content)
        .expect("Failed to write to gradle.properties");
}
