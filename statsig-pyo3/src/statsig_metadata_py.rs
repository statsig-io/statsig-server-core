use pyo3::prelude::*;
use pyo3::types::PyModule;
use sigstat::statsig_metadata::StatsigMetadata;

pub fn update_statsig_metadata(m: &Bound<'_, PyModule>) {
    let version = get_python_version(m).unwrap_or("unknown".to_string());

    StatsigMetadata::update_values(
        "statsig-server-core-python".to_string(),
        get_os(),
        get_arch(),
        version,
    );
}

fn get_python_version(m: &Bound<'_, PyModule>) -> PyResult<String> {
    let sys = PyModule::import(m.py(), "sys")?;
    let version = sys.getattr("version")?;
    version.extract::<String>()
}

fn get_os() -> String {
    let os = std::env::consts::OS;
    os.to_string()
}

fn get_arch() -> String {
    let arch = std::env::consts::ARCH;
    arch.to_string()
}
