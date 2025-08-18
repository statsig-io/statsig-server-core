mod data_store_base_py;
mod observability_client_base_py;
mod output_logger_provider_base_py;
mod pyo_utils;
mod safe_gil;
mod statsig_base_py;
mod statsig_metadata_py;
mod statsig_options_py;
mod statsig_persistent_storage_override_adapter_py;
mod statsig_types_py;
mod statsig_user_py;
mod unit_id_py;
mod valid_primitives_py;

use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;
use safe_gil::{notify_python_fork, notify_python_shutdown};

#[pymodule]
fn statsig_python_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    statsig_metadata_py::update_statsig_metadata(m);

    m.add_class::<statsig_base_py::StatsigBasePy>()?;
    m.add_class::<statsig_user_py::StatsigUserPy>()?;
    m.add_class::<statsig_options_py::StatsigOptionsPy>()?;
    m.add_class::<statsig_options_py::ProxyConfigPy>()?;
    m.add_class::<statsig_types_py::FeatureGatePy>()?;
    m.add_class::<statsig_types_py::DynamicConfigPy>()?;
    m.add_class::<statsig_types_py::ExperimentPy>()?;
    m.add_class::<statsig_types_py::LayerPy>()?;
    m.add_class::<statsig_types_py::FeatureGateEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::ExperimentEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::DynamicConfigEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::LayerEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::ParameterStoreEvaluationOptionsPy>()?;
    m.add_class::<statsig_types_py::InitializeDetailsPy>()?;
    m.add_class::<statsig_types_py::FailureDetailsPy>()?;
    m.add_class::<statsig_types_py::EvaluationDetailsPy>()?;
    m.add_class::<observability_client_base_py::ObservabilityClientBasePy>()?;
    m.add_class::<data_store_base_py::DataStoreBasePy>()?;
    m.add_class::<statsig_persistent_storage_override_adapter_py::PersistentStorageBasePy>()?;
    m.add_class::<output_logger_provider_base_py::OutputLoggerProviderBasePy>()?;
    m.add_class::<statsig_options_py::SpecAdapterConfigPy>()?;
    m.add_function(wrap_pyfunction!(notify_python_shutdown, m)?)?;
    m.add_function(wrap_pyfunction!(notify_python_fork, m)?)?;

    Ok(())
}

define_stub_info_gatherer!(stub_info);
