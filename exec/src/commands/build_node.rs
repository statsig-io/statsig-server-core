use crate::utils::*;
use colored::*;

pub fn execute(
    target: &Option<String>,
    build_for_release: &bool,
    use_napi_cross: &bool,
    cross_compile: &bool,
    vendored_openssl: &bool,
) {
    print_title("🛠️ ", "Building Node", Color::Yellow);

    ensure_empty_dir("build/node");
    println!("Emptied directory {}", "build/node".bold());

    let napi_version = get_napi_version();
    println!("Using\n - napi-cli: {}", napi_version);

    println!("\nBuilding...");
    run_napi_build(
        target,
        build_for_release,
        use_napi_cross,
        cross_compile,
        vendored_openssl,
    );

    add_custom_message_to_js();

    prettify_js_files();

    run_typescript();

    print_title("✅", "Successfully Built statsig-napi", Color::Green);
}

fn get_napi_version() -> String {
    let napi_version = std::process::Command::new(get_npm_command())
        .current_dir("statsig-napi")
        .args(["list", "@napi-rs/cli", "--depth=0"])
        .output()
        .expect("Failed to get napi version");

    let binding = String::from_utf8_lossy(&napi_version.stdout);
    let version = binding
        .lines()
        .find(|line| line.contains("@napi-rs/cli@"))
        .and_then(|line| line.split('@').last())
        .unwrap_or("unknown");

    version.to_string()
}

fn run_napi_build(
    target: &Option<String>,
    build_for_release: &bool,
    use_napi_cross: &bool,
    cross_compile: &bool,
    vendored_openssl: &bool,
) {
    let mut args: Vec<&str> = vec![
        "napi",
        "build",
        "--platform",
        "--js",
        "bindings.js",
        "--dts",
        "bindings.d.ts",
        "--output-dir",
        "./src",
        "--strip",
    ];

    if let Some(target) = target {
        args.push("--target");
        args.push(target);
    }

    if *use_napi_cross {
        args.push("--use-napi-cross");
    }

    if *cross_compile {
        args.push("--cross-compile");
    }

    if *vendored_openssl {
        args.push("--features");
        args.push("vendored_openssl");
    }

    if *build_for_release {
        args.push("--release");
    }

    let status = std::process::Command::new(get_npx_command())
        .current_dir("statsig-napi")
        .args(args)
        .status()
        .expect("Failed to execute napi build command");

    if !status.success() {
        panic!("napi build failed");
    }
}

fn add_custom_message_to_js() {
    print_title("🔧", "Generating JS Files", Color::Blue);

    let status = std::process::Command::new(get_npx_command())
        .current_dir("statsig-napi")
        .args([
            "jscodeshift",
            "--fail-on-error",
            "-t",
            "codemod/custom-error-message.js",
            "src/bindings.js",
        ])
        .status()
        .expect("Failed to execute jscodeshift command");

    if !status.success() {
        panic!("jscodeshift failed");
    }
}

fn prettify_js_files() {
    print_title("✨", "Prettifying Files", Color::Blue);

    let status = std::process::Command::new(get_npx_command())
        .current_dir("statsig-napi")
        .args(["prettier", "src/bindings.d.ts", "--write"])
        .status()
        .expect("Failed to execute prettier command");

    if !status.success() {
        panic!("prettier failed on bindings.d.ts");
    }

    let status = std::process::Command::new(get_npx_command())
        .current_dir("statsig-napi")
        .args(["prettier", "src/bindings.js", "--write"])
        .status()
        .expect("Failed to execute prettier command");

    if !status.success() {
        panic!("prettier failed on bindings.js");
    }
}

fn run_typescript() {
    print_title("🔧", "Running Typescript", Color::Blue);

    let status = std::process::Command::new(get_npx_command())
        .current_dir("statsig-napi")
        .args(["tsc"])
        .status()
        .expect("Failed to execute tsc command");

    if !status.success() {
        panic!("tsc failed");
    }

    println!("{}", "tsc succeeded".green());
}

fn get_npm_command() -> String {
    if cfg!(target_os = "windows") {
        "npm.cmd".to_string()
    } else {
        "npm".to_string()
    }
}

fn get_npx_command() -> String {
    if cfg!(target_os = "windows") {
        "npx.cmd".to_string()
    } else {
        "npx".to_string()
    }
}
