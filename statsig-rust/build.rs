fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("STATSIG_BUILD_PROTO").unwrap_or_default() == "true" {
        tonic_prost_build::Config::new()
            .out_dir("src/specs_response")
            .compile_protos(
                &["../api-interface-definitions/protos/config_specs.proto"],
                &["../api-interface-definitions/protos"],
            )?;
    }

    Ok(())
}
