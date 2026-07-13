use serde::{Deserialize, Serialize};

use crate::UserPersistedValues;

pub enum AnyEvaluationOptions {
    FeatureGateEvaluationOptions(FeatureGateEvaluationOptions),
    DynamicConfigEvaluationOptions(DynamicConfigEvaluationOptions),
    ExperimentEvaluationOptions(ExperimentEvaluationOptions),
    LayerEvaluationOptions(LayerEvaluationOptions),
    ParameterStoreEvaluationOptions(ParameterStoreEvaluationOptions),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FeatureGateEvaluationOptions {
    #[serde(default)]
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DynamicConfigEvaluationOptions {
    #[serde(default)]
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExperimentEvaluationOptions {
    #[serde(default)]
    pub disable_exposure_logging: bool,
    pub user_persisted_values: Option<UserPersistedValues>,
    // When a sticky value is applied, let a matching console override rule
    // take precedence over the persisted value.
    #[serde(default)]
    pub enforce_overrides: bool,
    // When a sticky value is applied, re-check targeting and drop the persisted
    // value if the user no longer passes targeting.
    #[serde(default)]
    pub enforce_targeting: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayerEvaluationOptions {
    #[serde(default)]
    pub disable_exposure_logging: bool,
    pub user_persisted_values: Option<UserPersistedValues>,
    // When a sticky value is applied, let a matching console override rule
    // take precedence over the persisted value.
    #[serde(default)]
    pub enforce_overrides: bool,
    // When a sticky value is applied, re-check targeting and drop the persisted
    // value if the user no longer passes targeting.
    #[serde(default)]
    pub enforce_targeting: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Copy)]
pub struct ParameterStoreEvaluationOptions {
    #[serde(default)]
    pub disable_exposure_logging: bool,
}

// -------------------------
//   From implementations into ParameterStoreEvaluationOptions
// -------------------------

impl From<ParameterStoreEvaluationOptions> for FeatureGateEvaluationOptions {
    fn from(opts: ParameterStoreEvaluationOptions) -> Self {
        FeatureGateEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging,
        }
    }
}

impl From<ParameterStoreEvaluationOptions> for DynamicConfigEvaluationOptions {
    fn from(opts: ParameterStoreEvaluationOptions) -> Self {
        DynamicConfigEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging,
        }
    }
}

impl From<ParameterStoreEvaluationOptions> for ExperimentEvaluationOptions {
    fn from(opts: ParameterStoreEvaluationOptions) -> Self {
        ExperimentEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging,
            user_persisted_values: None,
            enforce_overrides: false,
            enforce_targeting: false,
        }
    }
}

impl From<ParameterStoreEvaluationOptions> for LayerEvaluationOptions {
    fn from(opts: ParameterStoreEvaluationOptions) -> Self {
        LayerEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging,
            user_persisted_values: None,
            enforce_overrides: false,
            enforce_targeting: false,
        }
    }
}
