mod evaluation_types_py;
mod statsig_py;
mod statsig_user_py;

use pyo3::prelude::*;

#[pymodule]
fn sigstat_python_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<statsig_py::StatsigPy>()?;
    m.add_class::<statsig_user_py::StatsigUserPy>()?;
    m.add_class::<evaluation_types_py::FeatureGatePy>()?;
    m.add_class::<evaluation_types_py::ExperimentPy>()?;
    Ok(())
}
