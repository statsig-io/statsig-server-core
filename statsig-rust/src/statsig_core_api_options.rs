use serde::Deserialize;

pub enum AnyEvaluationOptions {
    FeatureGateEvaluationOptions(FeatureGateEvaluationOptions),
    DynamicConfigEvaluationOptions(DynamicConfigEvaluationOptions),
    ExperimentEvaluationOptions(ExperimentEvaluationOptions),
    LayerEvaluationOptions(LayerEvaluationOptions),
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
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayerEvaluationOptions {
    #[serde(default)]
    pub disable_exposure_logging: bool,
}
