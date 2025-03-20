use pyo3_stub_gen::Result;
use std::fs;

const STUB_FILE_SOURCE: &str = "py_src/statsig_python_core.pyi";
const STUB_FILE_DESTINATION: &str = "py_src/statsig_python_core/statsig_python_core.pyi";
fn main() -> Result<()> {
    let stub = statsig_python_core::stub_info()?;
    stub.generate()?;

    if !std::path::Path::new(STUB_FILE_SOURCE).exists() {
        panic!("statsig_python_core.pyi not found");
    }
    fs::rename(STUB_FILE_SOURCE, STUB_FILE_DESTINATION)?;

    Ok(())
}
