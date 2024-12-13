mod statsig_options_py;
mod statsig_py;
mod statsig_types_py;
mod statsig_user_py;

use pyo3::prelude::*;

#[pymodule]
fn sigstat_python_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<statsig_py::StatsigPy>()?;
    m.add_class::<statsig_user_py::StatsigUserPy>()?;
    m.add_class::<statsig_options_py::StatsigOptionsPy>()?;
    m.add_class::<statsig_types_py::FeatureGatePy>()?;
    m.add_class::<statsig_types_py::DynamicConfigPy>()?;
    m.add_class::<statsig_types_py::ExperimentPy>()?;
    m.add_class::<statsig_types_py::LayerPy>()?;
    Ok(())
}
