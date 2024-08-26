use std::env;
fn main() {
    run_c_bindgen();
    run_csharp_bindgen()
}

fn run_c_bindgen() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("crate_dir {}", crate_dir);

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
        .csharp_use_function_pointer(false)
        .csharp_class_name("StatsigFFI")
        .csharp_namespace("StatsigServer")
        .csharp_dll_name(
            "/Users/danielloomb/Projects/statsig-server-core/target/release/libstatsig_ffi.dylib",
        )
        .generate_csharp_file("./bindings/dotnet/StatsigFFI.g.cs")
        .unwrap();
}
