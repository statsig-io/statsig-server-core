fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `cargo package` needs the proto to be in the src directory
    if std::env::var("STATSIG_PUBLISH_RUST").is_ok() {
        tonic_prost_build::Config::new()
            .out_dir("src/specs_response")
            .compile_protos(&["src/protos/config_specs.proto"], &["src/protos"])?;

        return Ok(());
    }

    tonic_prost_build::Config::new()
        .out_dir("src/specs_response")
        .compile_protos(
            &["../api-interface-definitions/protos/config_specs.proto"],
            &["../api-interface-definitions/protos"],
        )?;

    Ok(())
}
