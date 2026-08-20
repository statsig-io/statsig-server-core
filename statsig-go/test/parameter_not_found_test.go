package test

import "testing"

func TestDynamicConfigContains(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	config := statsig.GetDynamicConfig(user, "test_email_config")

	if !config.EvaluationDetails.IsRecognized() {
		t.Errorf("test_email_config should be recognized, reason '%s'", config.EvaluationDetails.Reason)
	}
	if !config.Contains("header_text") {
		t.Errorf("config should contain header_text")
	}
	if config.Contains("not_a_real_key") {
		t.Errorf("config should not contain not_a_real_key")
	}
}

func TestExperimentContains(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	experiment := statsig.GetExperiment(user, "exp_with_obj_and_array")

	if !experiment.EvaluationDetails.IsRecognized() {
		t.Errorf("exp_with_obj_and_array should be recognized, reason '%s'", experiment.EvaluationDetails.Reason)
	}
	if !experiment.Contains("obj_param") {
		t.Errorf("experiment should contain obj_param")
	}
	if experiment.Contains("not_a_real_key") {
		t.Errorf("experiment should not contain not_a_real_key")
	}
}

func TestLayerContains(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	layer := statsig.GetLayer(user, "layer_with_many_params")

	if !layer.EvaluationDetails.IsRecognized() {
		t.Errorf("layer_with_many_params should be recognized, reason '%s'", layer.EvaluationDetails.Reason)
	}
	if !layer.Contains("a_string") {
		t.Errorf("layer should contain a_string")
	}
	if layer.Contains("not_a_real_key") {
		t.Errorf("layer should not contain not_a_real_key")
	}
}

func TestParameterStoreContains(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	store := statsig.GetParameterStore(user, "test_parameter_store")

	if !store.EvaluationDetails.IsRecognized() {
		t.Errorf("test_parameter_store should be recognized, reason '%s'", store.EvaluationDetails.Reason)
	}
	if !store.Contains("bool_param") {
		t.Errorf("store should contain bool_param, parameters: %v", store.GetParameterList())
	}
	if store.Contains("missing_param") {
		t.Errorf("store should not contain missing_param")
	}
	if len(store.GetParameterList()) == 0 {
		t.Errorf("store parameter list should not be empty")
	}
}

func TestUnrecognizedEntitiesAreNotRecognized(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	gate := statsig.GetFeatureGate(user, "not_a_real_gate")
	if gate.EvaluationDetails.IsRecognized() {
		t.Errorf("unknown gate should not be recognized, reason '%s'", gate.EvaluationDetails.Reason)
	}

	config := statsig.GetDynamicConfig(user, "not_a_real_config")
	if config.EvaluationDetails.IsRecognized() {
		t.Errorf("unknown config should not be recognized, reason '%s'", config.EvaluationDetails.Reason)
	}
	if config.Contains("anything") {
		t.Errorf("unknown config should not contain any key")
	}

	store := statsig.GetParameterStore(user, "not_a_real_store")
	if store.EvaluationDetails.IsRecognized() {
		t.Errorf("unknown store should not be recognized, reason '%s'", store.EvaluationDetails.Reason)
	}
	if store.Contains("anything") {
		t.Errorf("unknown store should not contain any parameter")
	}
	if len(store.GetParameterList()) != 0 {
		t.Errorf("unknown store parameter list should be empty, got %v", store.GetParameterList())
	}

	knownGate := statsig.GetFeatureGate(user, "test_public")
	if !knownGate.EvaluationDetails.IsRecognized() {
		t.Errorf("test_public should be recognized, reason '%s'", knownGate.EvaluationDetails.Reason)
	}
}
