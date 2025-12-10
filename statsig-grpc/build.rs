fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::fs::exists("../api-interface-definitions/protos/statsig_forward_proxy.proto")
        .unwrap_or(false)
    {
        std::fs::copy(
            "../api-interface-definitions/protos/statsig_forward_proxy.proto",
            "src/protos/statsig_forward_proxy.proto",
        )?;
    }

    tonic_build::compile_protos("src/protos/statsig_forward_proxy.proto")?;

    Ok(())
}
