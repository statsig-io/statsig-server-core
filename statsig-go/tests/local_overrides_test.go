package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestLayerOverrides(t *testing.T) {
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "layer_with_many_params"
	layerBeforeOverride := s.GetLayer(*user, layer_name, layerOptions)

	overrideValues := map[string]interface{}{
		"a_string": "overridden value",
	}

	if layerBeforeOverride.Value["a_string"] != "test_1" {
		t.Errorf("expected Value['a_string'] to be layer, got %v", layerBeforeOverride.Value["a_string"])
	}

	s.OverrideLayer(layer_name, overrideValues, "test-user-id")
	layerAfterOverride := s.GetLayer(*user, layer_name, layerOptions)

	if layerAfterOverride.Value["a_string"] != "overridden value" {
		t.Errorf("expected Value['a_string'] to be overridden value, got %v", layerAfterOverride.Value["a_string"])
	}

	s.RemoveLayerOverride(layer_name, "test-user-id")

	layerAfterRemoveOverride := s.GetLayer(*user, layer_name, layerOptions)
	if layerAfterRemoveOverride.Value["a_string"] != "test_1" {
		t.Errorf("expected Value['a_string'] to be layer, got %v", layerAfterRemoveOverride.Value["a_string"])
	}
}

func TestLayerOverridesWithCustomIDs(t *testing.T) {

	custom := map[string]string{
		"employee_id": "employee_id:12345",
	}

	user := statsig.NewStatsigUserBuilder().
		WithUserID("test-user-id").
		WithCustomIds(custom).
		Build()

	_, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "layer_with_many_params"
	layerBeforeOverride := s.GetLayer(*user, layer_name, layerOptions)

	overrideValues := map[string]interface{}{
		"a_string": "overridden value",
	}

	if layerBeforeOverride.Value["a_string"] != "test_1" {
		t.Errorf("expected Value['a_string'] to be layer, got %v", layerBeforeOverride.Value["a_string"])
	}

	s.OverrideLayer(layer_name, overrideValues, "employee_id:12345")
	layerAfterOverride := s.GetLayer(*user, layer_name, layerOptions)

	if layerAfterOverride.Value["a_string"] != "overridden value" {
		t.Errorf("expected Value['a_string'] to be overridden value, got %v", layerAfterOverride.Value["a_string"])
	}
}

func TestGateOverride(t *testing.T) {

	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}

	feature_gate := "test_public"
	gateBeforeOverride := s.GetFeatureGate(*user, feature_gate, checkGateOptions)

	overrideValues := false

	if gateBeforeOverride.Value != true {
		t.Errorf("expected Value to be true, got %v", gateBeforeOverride.Value)
	}

	s.OverrideGate(feature_gate, overrideValues, "test-user-id")
	gateAfterOverride := s.GetFeatureGate(*user, feature_gate, checkGateOptions)

	if gateAfterOverride.Value != false {
		t.Errorf("expected Value to be false, got %v", gateAfterOverride.Value)
	}

	s.RemoveGateOverride(feature_gate, "test-user-id")

	gateAfterRemoveOverride := s.GetFeatureGate(*user, feature_gate, checkGateOptions)
	if gateAfterRemoveOverride.Value != true {
		t.Errorf("expected Value to be true, got %v", gateAfterRemoveOverride.Value)
	}

}

func TestGateOverrideWithCustomIDs(t *testing.T) {

	custom := map[string]string{
		"employee_id": "employee_id:12345",
	}

	user := statsig.NewStatsigUserBuilder().
		WithUserID("test-user-id").
		WithCustomIds(custom).
		Build()

	_, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}

	feature_gate := "test_public"
	gateBeforeOverride := s.GetFeatureGate(*user, feature_gate, checkGateOptions)

	overrideValues := false

	if gateBeforeOverride.Value != true {
		t.Errorf("expected Value to be true, got %v", gateBeforeOverride.Value)
	}

	s.OverrideGate(feature_gate, overrideValues, "employee_id:12345")
	gateAfterOverride := s.GetFeatureGate(*user, feature_gate, checkGateOptions)

	if gateAfterOverride.Value != false {
		t.Errorf("expected Value to be false, got %v", gateAfterOverride.Value)
	}
}

func TestDynamicConfigOverride(t *testing.T) {

	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: false}

	dynamic_config := "big_number"
	configBeforeOverride := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	overrideValues := map[string]interface{}{
		"foo": -1.23,
	}

	if configBeforeOverride.Value["foo"] != 1e+21 {
		t.Errorf("expected Value['foo'] to be 1e+21, got %v", configBeforeOverride.Value["foo"])
	}

	s.OverrideDynamicConfig(dynamic_config, overrideValues, "test-user-id")
	configAfterOverride := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	if configAfterOverride.Value["foo"] != -1.23 {
		t.Errorf("expected Value['foo'] to be -1.23, got %v", configAfterOverride.Value["foo"])
	}

	s.RemoveDynamicConfigOverride(dynamic_config, "test-user-id")

	configAfterRemoveOverride := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	if configAfterRemoveOverride.Value["foo"] != 1e+21 {
		t.Errorf("expected Value['foo'] to be 1e+21, got %v", configAfterRemoveOverride.Value["foo"])
	}

}

func TestDynamicConfigOverrideWithCustomIDs(t *testing.T) {

	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: false}

	dynamic_config := "big_number"
	configBeforeOverride := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	overrideValues := map[string]interface{}{
		"foo": -1.23,
	}

	if configBeforeOverride.Value["foo"] != 1e+21 {
		t.Errorf("expected Value['foo'] to be 1e+21, got %v", configBeforeOverride.Value["foo"])
	}

	s.OverrideDynamicConfig(dynamic_config, overrideValues, "test-user-id")
	configAfterOverride := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	if configAfterOverride.Value["foo"] != -1.23 {
		t.Errorf("expected Value['foo'] to be -1.23, got %v", configAfterOverride.Value["foo"])
	}

	s.RemoveDynamicConfigOverride(dynamic_config, "test-user-id")

	configAfterRemoveOverride := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	if configAfterRemoveOverride.Value["foo"] != 1e+21 {
		t.Errorf("expected Value['foo'] to be 1e+21, got %v", configAfterRemoveOverride.Value["foo"])
	}

}

func TestExperimentOverride(t *testing.T) {

	custom := map[string]string{
		"employee_id": "employee_id:12345",
	}

	user := statsig.NewStatsigUserBuilder().
		WithUserID("test-user-id").
		WithCustomIds(custom).
		Build()

	_, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	experimentOptions := &statsig.GetExperimentOptions{DisableExposureLogging: false}

	experimentName := "experiment_with_many_params"
	experimentBeforeOverride := s.GetExperiment(*user, experimentName, experimentOptions)

	overrideValues := map[string]interface{}{
		"a_string": "overridden value",
	}

	if experimentBeforeOverride.Value["a_string"] != "test_1" {
		t.Errorf("expected Value['a_string'] to be test_1, got %v", experimentBeforeOverride.Value["a_string"])
	}

	s.OverrideExperiment(experimentName, overrideValues, "employee_id:12345")
	experimentAfterOverride := s.GetExperiment(*user, experimentName, experimentOptions)

	if experimentAfterOverride.Value["a_string"] != "overridden value" {
		t.Errorf("expected Value['a_string'] to be overridden value, got %v", experimentAfterOverride.Value["a_string"])
	}
}

func TestExperimentOverrideByGroupName(t *testing.T) {
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	experimentOptions := &statsig.GetExperimentOptions{DisableExposureLogging: false}

	experimentName := "experiment_with_many_params"
	experimentBeforeOverride := s.GetExperiment(*user, experimentName, experimentOptions)

	groupName := "Control"

	if experimentBeforeOverride.Value["a_string"] != "test_1" {
		t.Errorf("expected Value['a_string'] to be test_1, got %v", experimentBeforeOverride.Value["a_string"])
	}

	s.OverrideExperimentByGroupName(experimentName, groupName, "test-user-id")
	experimentAfterOverride := s.GetExperiment(*user, experimentName, experimentOptions)

	if experimentAfterOverride.Value["a_string"] != "control" {
		t.Errorf("expected Value['a_string'] to be control, got %v", experimentAfterOverride.Value["a_string"])
	}

}

func TestRemoveAllOverrides(t *testing.T) {
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	gateName := "test_public"
	originalGate := s.GetFeatureGate(*user, gateName, &statsig.CheckGateOptions{DisableExposureLogging: false})
	if originalGate.Value != true {
		t.Errorf("expected original gate value to be true, got %v", originalGate.Value)
	}
	s.OverrideGate(gateName, false, "test-user-id")

	configName := "big_number"
	originalConfig := s.GetDynamicConfig(*user, configName, &statsig.GetDynamicConfigOptions{DisableExposureLogging: false})
	if originalConfig.Value["foo"] != 1e+21 {
		t.Errorf("expected original config value to be 1e+21, got %v", originalConfig.Value["foo"])
	}
	s.OverrideDynamicConfig(configName, map[string]interface{}{"foo": -1.0}, "test-user-id")

	layerName := "layer_with_many_params"
	originalLayer := s.GetLayer(*user, layerName, &statsig.GetLayerOptions{DisableExposureLogging: false})
	if originalLayer.Value["a_string"] != "test_1" {
		t.Errorf("expected original layer value to be test_1, got %v", originalLayer.Value["a_string"])
	}
	s.OverrideLayer(layerName, map[string]interface{}{"a_string": "override_value"}, "test-user-id")

	experimentName := "experiment_with_many_params"
	originalExperiment := s.GetExperiment(*user, experimentName, &statsig.GetExperimentOptions{DisableExposureLogging: false})
	if originalExperiment.Value["a_string"] != "test_1" {
		t.Errorf("expected original experiment value to be test_1, got %v", originalExperiment.Value["a_string"])
	}
	s.OverrideExperiment(experimentName, map[string]interface{}{"a_string": "override_value"}, "test-user-id")

	if s.GetFeatureGate(*user, gateName, nil).Value != false {
		t.Errorf("expected overridden gate value to be false")
	}
	if s.GetDynamicConfig(*user, configName, nil).Value["foo"] != -1.0 {
		t.Errorf("expected overridden config value to be -1.0")
	}
	if s.GetLayer(*user, layerName, nil).Value["a_string"] != "override_value" {
		t.Errorf("expected overridden layer value to be 'override_value'")
	}
	if s.GetExperiment(*user, experimentName, nil).Value["a_string"] != "override_value" {
		t.Errorf("expected overridden experiment value to be 'override_value'")
	}

	s.RemoveAllOverrides()

	if s.GetFeatureGate(*user, gateName, nil).Value != true {
		t.Errorf("expected gate value after removal to be true")
	}
	if s.GetDynamicConfig(*user, configName, nil).Value["foo"] != 1e+21 {
		t.Errorf("expected config value after removal to be 1e+21")
	}
	if s.GetLayer(*user, layerName, nil).Value["a_string"] != "test_1" {
		t.Errorf("expected layer value after removal to be test_1")
	}
	if s.GetExperiment(*user, experimentName, nil).Value["a_string"] != "test_1" {
		t.Errorf("expected experiment value after removal to be test_1")
	}
}

func TestCustomIdOverridePrecedence(t *testing.T) {
	custom := map[string]string{
		"employee_id": "employee_id:12345",
	}

	user := statsig.NewStatsigUserBuilder().
		WithUserID("test-user-id").
		WithCustomIds(custom).
		Build()

	_, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "test-user-id", nil)

	defer teardown()

	feature_gate := "test_public"

	s.OverrideGate("test_public", false, "")
	s.OverrideGate("test_public", true, "employee_id:12345")
	s.OverrideGate("test_public", false, "test-user-id")

	if s.CheckGate(*user, feature_gate, nil) {
		t.Errorf("expected gate value after overrides to be false but got %v", s.CheckGate(*user, feature_gate, nil))
	}
}
