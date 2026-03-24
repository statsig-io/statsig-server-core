use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::PyBytes;
use pyo3::types::PyType;
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
    #[classmethod]
    pub fn preload(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyBytes>) -> PyResult<()> {
        let bytes: &[u8] = data.as_bytes();

        if let Err(e) = InternedStore::preload(bytes) {
            log_e!(TAG, "Failed to preload interned store: {}", e);
            return Err(PyRuntimeError::new_err(e.to_string()));
        }

        Ok(())
    }

    #[classmethod]
    pub fn preload_multi(_cls: &Bound<'_, PyType>, data: Vec<Bound<'_, PyBytes>>) -> PyResult<()> {
        let bytes: Vec<&[u8]> = data.iter().map(|data| data.as_bytes()).collect();

        if let Err(e) = InternedStore::preload_multi(&bytes) {
            log_e!(TAG, "Failed to preload interned store: {}", e);
            return Err(PyRuntimeError::new_err(e.to_string()));
        }

        Ok(())
    }

    #[classmethod]
    pub fn write_mmap_data(
        _cls: &Bound<'_, PyType>,
        data: Vec<Bound<'_, PyBytes>>,
        path: &str,
    ) -> PyResult<()> {
        let bytes: Vec<&[u8]> = data.iter().map(|data| data.as_bytes()).collect();

        if let Err(e) = InternedStore::write_mmap_data(&bytes, path) {
            log_e!(TAG, "Failed to preload mmap: {}", e);
            return Err(PyRuntimeError::new_err(e.to_string()));
        }

        Ok(())
    }

    #[classmethod]
    pub fn preload_mmap(_cls: &Bound<'_, PyType>, path: &str) -> PyResult<()> {
        if let Err(e) = InternedStore::preload_mmap(path) {
            log_e!(TAG, "Failed to load mmap data: {}", e);
            return Err(PyRuntimeError::new_err(e.to_string()));
        }

        Ok(())
    }
}
