fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("api-interface-definitions/protos/statsig_forward_proxy.proto")?;
    Ok(())
}
