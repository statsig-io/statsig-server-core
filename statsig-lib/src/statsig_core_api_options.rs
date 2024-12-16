pub enum AnyEvaluationOptions {
    FeatureGateEvaluationOptions(FeatureGateEvaluationOptions),
    DynamicConfigEvaluationOptions(DynamicConfigEvaluationOptions),
    ExperimentEvaluationOptions(ExperimentEvaluationOptions),
    LayerEvaluationOptions(LayerEvaluationOptions),
}

#[derive(Debug, Clone, Default)]
pub struct FeatureGateEvaluationOptions {
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DynamicConfigEvaluationOptions {
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ExperimentEvaluationOptions {
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LayerEvaluationOptions {
    pub disable_exposure_logging: bool,
}
