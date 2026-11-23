package statsig_go_core

import "encoding/json"

// ExperimentEvaluationOptionsV2 opts into structured persisted parameters.
// A nil UserPersistedValues pointer does not opt into persistence; a pointer to
// an empty map opts in with no prior assignments, as with the legacy options.
type ExperimentEvaluationOptionsV2 struct {
	DisableExposureLogging bool                   `json:"disable_exposure_logging"`
	UserPersistedValues    *UserPersistedValuesV2 `json:"user_persisted_values,omitempty"`
	EnforceOverrides       bool                   `json:"enforce_overrides"`
	EnforceTargeting       bool                   `json:"enforce_targeting"`
}

// LayerEvaluationOptionsV2 is the layer equivalent of ExperimentEvaluationOptionsV2.
type LayerEvaluationOptionsV2 struct {
	DisableExposureLogging bool                   `json:"disable_exposure_logging"`
	UserPersistedValues    *UserPersistedValuesV2 `json:"user_persisted_values,omitempty"`
	EnforceOverrides       bool                   `json:"enforce_overrides"`
	EnforceTargeting       bool                   `json:"enforce_targeting"`
}

// GetExperimentWithOptionsV2 evaluates with typed persistence. Evaluation result
// numbers and getters retain their existing behavior.
func (s *Statsig) GetExperimentWithOptionsV2(user *StatsigUser, name string, options *ExperimentEvaluationOptionsV2) Experiment {
	return s.getExperimentWithOptions(user, name, options)
}

// GetLayerWithOptionsV2 evaluates a layer with typed persistence.
func (s *Statsig) GetLayerWithOptionsV2(user *StatsigUser, name string, options *LayerEvaluationOptionsV2) Layer {
	return s.getLayerWithOptions(user, name, options)
}

// Both option versions serialize to the same native schema. A typed nil
// pointer retains the legacy empty-options representation rather than "null".
func marshalPersistentEvaluationOptions(options any) (string, error) {
	data, err := json.Marshal(options)
	if err != nil {
		return "", err
	}
	if string(data) == "null" {
		return "", nil
	}
	return string(data), nil
}
