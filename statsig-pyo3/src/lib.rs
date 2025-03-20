mod observability_client_py;
mod pyo_utils;
mod statsig_metadata_py;
mod statsig_options_py;
mod statsig_py;
mod statsig_types_py;
mod statsig_user_py;

use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

#[pymodule]
fn statsig_python_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    statsig_metadata_py::update_statsig_metadata(m);

    m.add_class::<statsig_py::StatsigPy>()?;
    m.add_class::<statsig_user_py::StatsigUserPy>()?;
    m.add_class::<statsig_options_py::StatsigOptionsPy>()?;
    m.add_class::<statsig_types_py::FeatureGatePy>()?;
    m.add_class::<statsig_types_py::DynamicConfigPy>()?;
    m.add_class::<statsig_types_py::ExperimentPy>()?;
    m.add_class::<statsig_types_py::LayerPy>()?;
    m.add_class::<statsig_types_py::FeatureGateEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::ExperimentEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::DynamicConfigEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::LayerEvaluationOptionsPy>()?;
    m.add_class::<observability_client_py::ObservabilityClientPy>()?;

    Ok(())
}

define_stub_info_gatherer!(stub_info);
