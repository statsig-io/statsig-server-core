package statsig_go_core

type FeatureGateEvaluationOptions struct {
	DisableExposureLogging bool `json:"disable_exposure_logging"`
}

type DynamicConfigEvaluationOptions struct {
	DisableExposureLogging bool `json:"disable_exposure_logging"`
}

type ExperimentEvaluationOptions struct {
	DisableExposureLogging bool                 `json:"disable_exposure_logging"`
	UserPersistedValues    *UserPersistedValues `json:"user_persisted_values,omitempty"`
}

type LayerEvaluationOptions struct {
	DisableExposureLogging bool                 `json:"disable_exposure_logging"`
	UserPersistedValues    *UserPersistedValues `json:"user_persisted_values,omitempty"`
}

type ParameterStoreEvaluationOptions struct {
	DisableExposureLogging bool `json:"disable_exposure_logging"`
}
