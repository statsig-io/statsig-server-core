use std::env;
fn main() {
    run_c_bindgen();
    run_csharp_bindgen();
    run_python_build();
    run_cpp_bindgen();
}

fn run_c_bindgen() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut config: cbindgen::Config = Default::default();
    config.header = Some(
        "typedef int Statsig;\ntypedef int StatsigOptions;\ntypedef int StatsigUser;".to_string(),
    );
    config.language = cbindgen::Language::C;
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate C bindings.")
        .write_to_file("include/statsig_ffi.h");
}

fn run_csharp_bindgen() {
    csbindgen::Builder::default()
        .input_extern_file("./src/lib.rs")
        .input_extern_file("./src/statsig_options_c.rs")
        .input_extern_file("./src/statsig_user_c.rs")
        .input_extern_file("./src/statsig_c.rs")
        .input_extern_file("./src/statsig_metadata_c.rs")
        .input_extern_file("./src/persistent_storage_c.rs")
        .input_extern_file("./src/ffi_utils.rs")
        .csharp_use_function_pointer(false)
        .csharp_class_name("StatsigFFI")
        .csharp_namespace("Statsig")
        .csharp_dll_name("libstatsig_ffi")
        .csharp_type_rename(|rust_type_name| match rust_type_name.as_str() {
            "SafeOptBool" => "int".into(),
            _ => rust_type_name,
        })
        .generate_csharp_file("../statsig-dotnet/src/Statsig/StatsigFFI.g.cs")
        .expect("Statsig Build Error: Failed to generate C# bindings.");
}

fn run_cpp_bindgen() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut config = cbindgen::Config::default();

    // Ensure the generated header is C++-friendly
    config.language = cbindgen::Language::C; // Still C, but we wrap with extern "C"
    config.cpp_compat = true; // This enables C++ compatibility

    // Generate the bindings
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate C bindings.")
        .write_to_file("../statsig-cpp/include/libstatsig_ffi.h");
}

fn run_python_build() {}
