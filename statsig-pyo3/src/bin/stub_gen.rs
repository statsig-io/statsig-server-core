use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = statsig_python_core::stub_info()?;
    stub.generate()?;
    Ok(())
}
