use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::*;

use statsig_rust::interned_values::InternedStore;
use statsig_rust::log_e;

const TAG: &str = stringify!(InternStorePy);

#[gen_stub_pyclass]
#[pyclass(name = "InternedStore", module = "statsig_python_core")]
#[derive(Default)]
pub struct InternedStorePy;

#[pymethods]
#[gen_stub_pymethods]
impl InternedStorePy {
    #[staticmethod]
    pub fn preload(data: &Bound<'_, PyBytes>) {
        let bytes: &[u8] = data.as_bytes();

        if let Err(e) = InternedStore::preload(bytes) {
            log_e!(TAG, "Failed to preload interned store: {}", e);
        }
    }

    #[staticmethod]
    pub fn preload_multi(data: Vec<Bound<'_, PyBytes>>) {
        let bytes: Vec<&[u8]> = data.iter().map(|data| data.as_bytes()).collect();

        if let Err(e) = InternedStore::preload_multi(&bytes) {
            log_e!(TAG, "Failed to preload interned store: {}", e);
        }
    }
}
