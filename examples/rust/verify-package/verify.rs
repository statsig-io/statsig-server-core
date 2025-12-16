use statsig_rust::{Statsig, StatsigUser};
use std::collections::HashMap;
use std::process::Command;

fn exec(command: &str, arg: &str) -> String {
    let output = Command::new(command)
        .arg(arg)
        .output()
        .expect("Failed to execute process");
    String::from_utf8(output.stdout).expect("Failed to convert output to string")
}

#[tokio::main]
async fn main() {
    let sdk_key = std::env::var("STATSIG_SERVER_SDK_KEY").unwrap();
    if sdk_key.is_empty() {
        panic!("STATSIG_SERVER_SDK_KEY is not set");
    }

    let statsig = Statsig::new(&sdk_key, None);
    statsig.initialize().await.unwrap();

    let os = exec("uname", "-s");
    let arch = exec("uname", "-m");
    let rust_version = exec("rustc", "--version");

    println!("OS: {}", os);
    println!("Arch: {}", arch);
    println!("Rust Version: {}", rust_version);

    let mut user = StatsigUser::with_user_id("a_user");
    user.set_custom(Some(HashMap::from([
        ("os".to_string(), os),
        ("arch".to_string(), arch),
        ("rustVersion".to_string(), rust_version),
    ])));

    let gate = statsig.check_gate(&user, "test_public");
    let gcir = statsig.get_client_init_response(&user);

    println!(
        "-------------------------------- Get Client Initialize Response --------------------------------"
    );
    println!("{}", serde_json::to_string_pretty(&gcir).unwrap());
    println!(
        "-------------------------------------------------------------------------------------------------"
    );

    println!("Gate test_public: {}", gate);

    if !gate {
        panic!("\"test_public\" gate is false but should be true");
    }

    if gcir.feature_gates.is_empty()
        || gcir.layer_configs.is_empty()
        || gcir.dynamic_configs.is_empty()
    {
        panic!("GCIR is missing required fields");
    }

    println!("All checks passed, shutting down...");
    statsig.shutdown().await.unwrap();
    println!("Shutdown complete");
}
