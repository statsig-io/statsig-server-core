use pyo3_stub_gen::Result;
use std::{fs, path::Path};

const STUB_FILE_SOURCE: &str = "./statsig-pyo3/py_src/statsig_python_core.pyi";
const STUB_FILE_DESTINATION: &str =
    "./statsig-pyo3/py_src/statsig_python_core/statsig_python_core.pyi";

fn main() -> Result<()> {
    let stub = statsig_python_core::stub_info()?;
    stub.generate()?;

    let stub_file_source = get_relative_path(STUB_FILE_SOURCE);
    if !Path::new(&stub_file_source).exists() {
        panic!("statsig_python_core.pyi not found at {stub_file_source}");
    }

    let destination = get_relative_path(STUB_FILE_DESTINATION);
    fs::rename(stub_file_source, destination)?;
    Ok(())
}

fn get_relative_path(path: &str) -> String {
    let current_file = std::env::current_exe().unwrap(); // assume <rootDir>/target/debug
    let root = current_file.ancestors().nth(3).unwrap();

    let path = root.join(path);
    path.to_str().unwrap().to_string()
}
