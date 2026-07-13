package test

import (
	"testing"
)

func contains(list []string, target string) bool {
	for _, item := range list {
		if item == target {
			return true
		}
	}
	return false
}

func TestGetFeatureGateList(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	gates := statsig.GetFeatureGateList()
	if len(gates) == 0 {
		t.Errorf("Feature gate list should not be empty")
	}
	if !contains(gates, "test_public") {
		t.Errorf("Feature gate list should contain 'test_public', got %v", gates)
	}
}

func TestGetDynamicConfigList(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	configs := statsig.GetDynamicConfigList()
	if len(configs) == 0 {
		t.Errorf("Dynamic config list should not be empty")
	}
	if !contains(configs, "test_email_config") {
		t.Errorf("Dynamic config list should contain 'test_email_config', got %v", configs)
	}
}

func TestGetExperimentList(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	experiments := statsig.GetExperimentList()
	if len(experiments) == 0 {
		t.Errorf("Experiment list should not be empty")
	}
	if !contains(experiments, "exp_with_obj_and_array") {
		t.Errorf("Experiment list should contain 'exp_with_obj_and_array', got %v", experiments)
	}
}

func TestGetAutotuneList(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	autotunes := statsig.GetAutotuneList()
	if len(autotunes) == 0 {
		t.Errorf("Autotune list should not be empty")
	}
	if !contains(autotunes, "test_autotune") {
		t.Errorf("Autotune list should contain 'test_autotune', got %v", autotunes)
	}
}

func TestGetParameterStoreList(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	stores := statsig.GetParameterStoreList()
	if len(stores) == 0 {
		t.Errorf("Parameter store list should not be empty")
	}
	if !contains(stores, "test_parameter_store") {
		t.Errorf("Parameter store list should contain 'test_parameter_store', got %v", stores)
	}
}
