fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::fs::exists("../api-interface-definitions/protos/statsig_forward_proxy.proto")
        .unwrap_or(false)
    {
        std::fs::copy(
            "../api-interface-definitions/protos/statsig_forward_proxy.proto",
            "src/protos/statsig_forward_proxy.proto",
        )?;
    }

    // Tonic emits an async_trait server interface whose Future return type is already must_use.
    // Scope the Rust 1.99 Clippy allowance to that generated server module.
    tonic_build::configure()
        .server_mod_attribute(
            "statsig_forward_proxy",
            "#[allow(clippy::double_must_use, clippy::mixed_attributes_style)]",
        )
        .compile_protos(&["src/protos/statsig_forward_proxy.proto"], &["src/protos"])?;

    Ok(())
}
