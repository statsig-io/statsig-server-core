use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Once;

const BUILD_DIR: &str = "../build";

static INIT: Once = Once::new();

pub fn initialize() {
    INIT.call_once(|| {
        if Path::new(BUILD_DIR).exists() {
            fs::remove_dir_all(BUILD_DIR).expect("Failed to remove existing directory");
        }

        Command::new("mkdir")
            .arg("-p")
            .arg(BUILD_DIR)
            .status()
            .expect("Failed to create directory");

        // Build the Rust library
        Command::new("cargo")
            .args(["build"])
            .current_dir("../")
            .status()
            .expect("Failed to build the statsig_c_lib library");
    });
}

// #[test]
// fn test_py_bindings() {
//     initialize();
//
//     Command::new("./build_python")
//         .current_dir("../tools")
//         .output()
//         .expect("Failed to run node example");
//
//     let output = Command::new("python3")
//         .args([
//             "../examples/example.py",
//         ])
//         .output()
//         .expect("Failed to run python example");
//
//     let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");
//     let err_str = String::from_utf8(output.stderr).expect("Failed to convert output to string");
//     assert_eq!(output_str.trim(), "Gate check passed!");
//     assert_eq!(err_str, "");
// }
//
// #[test]
// fn test_c_bindings() {
//     initialize();
//
//     // Compile the C example
//     Command::new("gcc")
//         .args([
//             "-o",
//             "../build/statsig_c_example",
//             "../examples/main.c",
//             "-L../target/debug",
//             "-lstatsig_ffi",
//             "-ldl",
//         ])
//         .status()
//         .expect("Failed to compile the C example");
//
//     // Run the C example
//     let output = Command::new("../build/statsig_c_example")
//         .output()
//         .expect("Failed to run the C example");
//
//     // Check the output
//     let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");
//     assert_eq!(output_str.trim(), "Gate check passed!");
//
//     // if Path::new(BUILD_DIR).exists() {
//     //     fs::remove_dir_all(BUILD_DIR).expect("Failed to remove existing directory");
//     // }
// }
//
// #[test]
// fn test_node_bindings() {
//     initialize();
//
//     Command::new("npm")
//         .current_dir("../examples/node")
//         .args(["install"])
//         .output()
//         .expect("Failed to run node example");
//
//     let output = Command::new("node")
//         .current_dir("../examples/node")
//         .args(["example.js"])
//         .output()
//         .expect("Failed to run node example");
//
//     let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");
//     assert_eq!(output_str.trim(), "Gate check passed!");
// }
