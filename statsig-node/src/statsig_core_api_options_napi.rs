use napi_derive::napi;
use statsig_rust::{
    DynamicConfigEvaluationOptions, ExperimentEvaluationOptions, FeatureGateEvaluationOptions,
    LayerEvaluationOptions,
};

// -------------------------
//   Feature Gate Options
// -------------------------

#[napi(object, js_name = "FeatureGateEvaluationOptions")]
pub struct FeatureGateEvaluationOptionsNapi {
    pub disable_exposure_logging: Option<bool>,
}

impl From<FeatureGateEvaluationOptionsNapi> for FeatureGateEvaluationOptions {
    fn from(opts: FeatureGateEvaluationOptionsNapi) -> Self {
        FeatureGateEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging.unwrap_or(false),
        }
    }
}

// -------------------------
//   Dynamic Config Options
// -------------------------

#[napi(object, js_name = "DynamicConfigEvaluationOptions")]
pub struct DynamicConfigEvaluationOptionsNapi {
    pub disable_exposure_logging: Option<bool>,
}

impl From<DynamicConfigEvaluationOptionsNapi> for DynamicConfigEvaluationOptions {
    fn from(opts: DynamicConfigEvaluationOptionsNapi) -> Self {
        DynamicConfigEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging.unwrap_or(false),
        }
    }
}

// -------------------------
//   Experiment Options
// -------------------------

#[napi(object, js_name = "ExperimentEvaluationOptions")]
pub struct ExperimentEvaluationOptionsNapi {
    pub disable_exposure_logging: Option<bool>,
}

impl From<ExperimentEvaluationOptionsNapi> for ExperimentEvaluationOptions {
    fn from(opts: ExperimentEvaluationOptionsNapi) -> Self {
        ExperimentEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging.unwrap_or(false),
        }
    }
}

// -------------------------
//   Layer Options
// -------------------------

#[napi(object, js_name = "LayerEvaluationOptions")]
pub struct LayerEvaluationOptionsNapi {
    pub disable_exposure_logging: Option<bool>,
}

impl From<LayerEvaluationOptionsNapi> for LayerEvaluationOptions {
    fn from(opts: LayerEvaluationOptionsNapi) -> Self {
        LayerEvaluationOptions {
            disable_exposure_logging: opts.disable_exposure_logging.unwrap_or(false),
        }
    }
}
