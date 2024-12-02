#[derive(Debug, Clone, Default)]
pub struct CheckGateOptions {
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GetFeatureGateOptions {
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GetExperimentOptions {
    pub disable_exposure_logging: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GetDynamicConfigOptions {
    pub disable_exposure_logging: bool,
}
