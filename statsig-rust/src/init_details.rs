use serde::Serialize;

use crate::{networking::NetworkError, SpecsInfo, SpecsSource, StatsigErr};

#[derive(Debug, Clone, Serialize)]
pub struct FailureDetails {
    pub reason: String,
    pub error: Option<StatsigErr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitializeDetails {
    pub duration_ms: u64,
    pub init_success: bool,
    pub is_config_spec_ready: bool,
    pub is_id_list_ready: Option<bool>,
    pub source: SpecsSource,
    pub failure_details: Option<FailureDetails>,
    pub spec_source_api: Option<String>,

    #[deprecated(since = "0.11.0", note = "Use the duration_ms field instead")]
    pub duration: f64,
}

impl Default for InitializeDetails {
    fn default() -> Self {
        InitializeDetails {
            duration_ms: 0,
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: SpecsSource::Uninitialized,
            failure_details: None,
            spec_source_api: None,

            #[allow(deprecated)]
            duration: 0.0,
        }
    }
}

impl InitializeDetails {
    pub fn new(
        init_success: bool,
        duration_ms: u64,
        specs_info: SpecsInfo,
        is_id_list_ready: Option<bool>,
        error: Option<StatsigErr>,
    ) -> InitializeDetails {
        let is_config_spec_ready = matches!(specs_info.lcut, Some(v) if v != 0);

        let failure_details =
            if let Some(StatsigErr::NetworkError(NetworkError::DisableNetworkOn(_))) = error {
                None
            } else {
                error.as_ref().map(|e| FailureDetails {
                    reason: e.to_string(),
                    error: Some(e.clone()),
                })
            };

        InitializeDetails {
            init_success,
            is_config_spec_ready,
            is_id_list_ready,
            source: specs_info.source.clone(),
            failure_details,
            duration_ms,
            spec_source_api: specs_info.source_api.clone(),

            #[allow(deprecated)]
            duration: duration_ms as f64,
        }
    }

    pub fn from_error(reason: &str, error: Option<StatsigErr>) -> Self {
        InitializeDetails {
            duration_ms: 0,
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: SpecsSource::Uninitialized,
            failure_details: Some(FailureDetails {
                reason: reason.to_string(),
                error,
            }),
            spec_source_api: None,

            #[allow(deprecated)]
            duration: 0.0,
        }
    }

    pub fn from_timeout_failure(timeout_ms: u64) -> InitializeDetails {
        InitializeDetails {
            init_success: false,
            is_config_spec_ready: false,
            is_id_list_ready: None,
            source: SpecsSource::Uninitialized,
            failure_details: Some(FailureDetails {
                reason: "Initialization timed out".to_string(),
                error: None,
            }),
            duration_ms: timeout_ms,
            spec_source_api: None,

            #[allow(deprecated)]
            duration: timeout_ms as f64,
        }
    }
}
