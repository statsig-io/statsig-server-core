fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `cargo package` needs the proto to be in the src directory
    if std::env::var("STATSIG_PUBLISH_RUST").is_ok() {
        tonic_build::compile_protos("src/protos/statsig_forward_proxy.proto")?;
    } else {
        tonic_build::compile_protos(
            "../api-interface-definitions/protos/statsig_forward_proxy.proto",
        )?;
    }
    Ok(())
}
