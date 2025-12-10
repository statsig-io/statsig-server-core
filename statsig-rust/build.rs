fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::fs::exists("../api-interface-definitions/protos/config_specs.proto").unwrap_or(false) {
        std::fs::copy(
            "../api-interface-definitions/protos/config_specs.proto",
            "src/protos/config_specs.proto",
        )?;
    }

    tonic_prost_build::Config::new()
        .out_dir("src/specs_response")
        .compile_protos(&["src/protos/config_specs.proto"], &["src/protos"])?;

    Ok(())
}
