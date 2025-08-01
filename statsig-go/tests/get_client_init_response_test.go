package tests

import (
	"reflect"
	"testing"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
	utils "github.com/statsig-io/statsig-server-core/statsig-go/src/utils"
)

func setupStatsigGCIR(t *testing.T, filterType string, filter []string, hash_algo statsig.HashAlgo) map[string]interface{} {
	client_key := "client-key"
	user := statsig.NewStatsigUserBuilder().
		WithUserID("9").
		WithAppVersion("1.3").
		WithUserAgent("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1").
		WithIpAddress("1.0.0.0").
		WithLocale("en_US").
		Build()

	_, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	gcirOptions := statsig.ClientInitResponseOptions{
		HashAlgo:     &hash_algo,
		ClientSDKKey: &client_key,
	}

	switch filterType {
	case "feature_gates":
		gcirOptions.FeatureGateFilter = &filter
	case "dynamic_configs":
		gcirOptions.DynamicConfigFilter = &filter
	case "layer_configs":
		gcirOptions.LayerFilter = &filter
	default:
	}

	defer teardown()

	res := s.GetClientInitializeResponse(*user, &gcirOptions)

	result, err := utils.ConvertStringToJSON[map[string]interface{}](res)
	if err != nil {
		return map[string]interface{}{}
	}

	return result
}
func TestFeatureGateFilter(t *testing.T) {
	result := setupStatsigGCIR(t, "feature_gates", []string{"test_public"}, statsig.NONE)

	gates := result["feature_gates"].(map[string]interface{})
	if len(gates) != 1 || gates["test_public"] == nil {
		t.Errorf("expected only 'test_public' in feature_gates, got: %+v", gates)
	}
}

func TestPublicGate(t *testing.T) {
	result := setupStatsigGCIR(t, "feature_gates", []string{"test_public"}, statsig.NONE)

	gates := result["feature_gates"].(map[string]interface{})
	gate := gates["test_public"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                "test_public",
		"value":               true,
		"rule_id":             "6X3qJgyfwA81IJ2dxI7lYp",
		"id_type":             "userID",
		"secondary_exposures": []interface{}{},
	}

	if !reflect.DeepEqual(gate, expected) {
		t.Errorf("test_public gate did not match expected. Got: %+v", gate)
	}
}

func TestPublicGateDJB2(t *testing.T) {
	result := setupStatsigGCIR(t, "feature_gates", []string{"test_public"}, statsig.DJB2)

	hashedVal := "3968762550"

	gates := result["feature_gates"].(map[string]interface{})
	gate := gates[hashedVal].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                hashedVal,
		"value":               true,
		"rule_id":             "6X3qJgyfwA81IJ2dxI7lYp",
		"id_type":             "userID",
		"secondary_exposures": []interface{}{},
	}

	if !reflect.DeepEqual(gate, expected) {
		t.Errorf("test_public gate did not match expected. Got: %+v", gate)
	}
}

func TestNestedGateCondition(t *testing.T) {
	result := setupStatsigGCIR(t, "feature_gates", []string{"test_nested_gate_condition"}, statsig.NONE)

	gates := result["feature_gates"].(map[string]interface{})
	gate := gates["test_nested_gate_condition"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":    "test_nested_gate_condition",
		"value":   true,
		"rule_id": "6MlXHRavmo1ujM1NkZNjhQ",
		"id_type": "userID",
		"secondary_exposures": []interface{}{
			map[string]interface{}{"gate": "test_email", "gateValue": "false", "ruleID": "default"},
			map[string]interface{}{"gate": "test_environment_tier", "gateValue": "false", "ruleID": "default"},
		},
	}

	if !reflect.DeepEqual(gate, expected) {
		t.Errorf("nested_gate_condition did not match expected. Got: %+v", gate)
	}
}

func TestTargetedExpInLayerWithHoldout(t *testing.T) {
	result := setupStatsigGCIR(t, "dynamic_configs", []string{"targeted_exp_in_layer_with_holdout"}, statsig.NONE)

	dcs := result["dynamic_configs"].(map[string]interface{})
	exp := dcs["targeted_exp_in_layer_with_holdout"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                  "targeted_exp_in_layer_with_holdout",
		"value":                 map[string]interface{}{"exp_val": "shipped_test", "layer_val": "layer_default"},
		"rule_id":               "layerAssignment",
		"group":                 "layerAssignment",
		"is_device_based":       false,
		"id_type":               "userID",
		"is_experiment_active":  true,
		"is_user_in_experiment": false,
		"is_in_layer":           true,
		"explicit_parameters":   []interface{}{"exp_val"},
		"secondary_exposures": []interface{}{
			map[string]interface{}{"gate": "global_holdout", "gateValue": "false", "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"},
			map[string]interface{}{"gate": "layer_holdout", "gateValue": "false", "ruleID": "2bAVp6R3C85vCYrR6be36n:10.00:5"},
		},
	}

	if !reflect.DeepEqual(exp, expected) {
		t.Errorf("targeted_exp_in_layer_with_holdout did not match expected. Got: %+v", exp)
	}
}

func TestTargetedExpInUnlayeredWithHoldout(t *testing.T) {
	result := setupStatsigGCIR(t, "dynamic_configs", []string{"targeted_exp_in_unlayered_with_holdout"}, statsig.NONE)

	dcs := result["dynamic_configs"].(map[string]interface{})
	cfg := dcs["targeted_exp_in_unlayered_with_holdout"].(map[string]interface{})

	expected := map[string]interface{}{
		"group":                 "targetingGate",
		"id_type":               "userID",
		"is_device_based":       false,
		"is_experiment_active":  true,
		"is_user_in_experiment": false,
		"name":                  "targeted_exp_in_unlayered_with_holdout",
		"rule_id":               "targetingGate",
		"value":                 map[string]interface{}{},
		"secondary_exposures": []interface{}{
			map[string]interface{}{"gate": "global_holdout", "gateValue": "false", "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"},
			map[string]interface{}{"gate": "exp_holdout", "gateValue": "false", "ruleID": "1rEqLOpCROaRafv7ubGgax"},
			map[string]interface{}{"gate": "test_50_50", "gateValue": "false", "ruleID": "6U5gYSQ2jRCDWvfPzKSQY9"},
		},
	}

	if !reflect.DeepEqual(cfg, expected) {
		t.Errorf("targeted_exp_in_unlayered_with_holdout did not match expected. Got:\n %+v", cfg)
	}
}

func TestExp5050Targeting(t *testing.T) {
	result := setupStatsigGCIR(t, "dynamic_configs", []string{"test_exp_5050_targeting"}, statsig.NONE)

	dcs := result["dynamic_configs"].(map[string]interface{})
	exp := dcs["test_exp_5050_targeting"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                  "test_exp_5050_targeting",
		"value":                 map[string]interface{}{},
		"rule_id":               "targetingGate",
		"group":                 "targetingGate",
		"is_device_based":       false,
		"id_type":               "userID",
		"is_experiment_active":  true,
		"is_user_in_experiment": false,
		"secondary_exposures": []interface{}{
			map[string]interface{}{"gate": "global_holdout", "gateValue": "false", "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"},
			map[string]interface{}{"gate": "test_50_50", "gateValue": "false", "ruleID": "6U5gYSQ2jRCDWvfPzKSQY9"},
		},
	}

	if !reflect.DeepEqual(exp, expected) {
		t.Errorf("test_exp_5050_targeting did not match expected. Got: %+v", exp)
	}
}

func TestTargetingWithCapitalLetterGate(t *testing.T) {
	result := setupStatsigGCIR(t, "dynamic_configs", []string{"test_targetting_with_capital_letter_gate"}, statsig.NONE)

	dcs := result["dynamic_configs"].(map[string]interface{})
	exp := dcs["test_targetting_with_capital_letter_gate"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                  "test_targetting_with_capital_letter_gate",
		"value":                 map[string]interface{}{"Result": "This is right"},
		"rule_id":               "74pyYBYPZ5Xly55E6J3lEq",
		"group":                 "74pyYBYPZ5Xly55E6J3lEq",
		"group_name":            "Test",
		"is_device_based":       false,
		"id_type":               "userID",
		"is_experiment_active":  true,
		"is_user_in_experiment": true,
		"secondary_exposures": []interface{}{
			map[string]interface{}{"gate": "global_holdout", "gateValue": "false", "ruleID": "3QoA4ncNdVGBaMt3N1KYjz:0.50:1"},
			map[string]interface{}{"gate": "test_putting_CAPITAL_letters_in_id", "gateValue": "true", "ruleID": "3Gv6T9YIObRmqZV5nAv0fO"},
		},
	}

	if !reflect.DeepEqual(exp, expected) {
		t.Errorf("test_targetting_with_capital_letter_gate did not match expected. Got: %+v", exp)
	}
}

func TestLayerWithManyParams(t *testing.T) {
	result := setupStatsigGCIR(t, "layer_configs", []string{"layer_with_many_params"}, statsig.NONE)

	layers := result["layer_configs"].(map[string]interface{})
	layer := layers["layer_with_many_params"].(map[string]interface{})

	expected := map[string]interface{}{
		"name": "layer_with_many_params",
		"value": map[string]interface{}{
			"a_string":       "layer",
			"another_string": "layer_default",
			"a_number":       float64(799),
			"a_bool":         false,
			"an_object":      map[string]interface{}{"value": "layer_default"},
			"an_array":       []interface{}{"layer_default"},
			"another_bool":   true,
			"another_number": float64(0),
		},
		"id_type":                         "userID",
		"rule_id":                         "default",
		"group":                           "default",
		"is_device_based":                 false,
		"explicit_parameters":             []interface{}{},
		"secondary_exposures":             []interface{}{},
		"undelegated_secondary_exposures": []interface{}{},
	}

	if !reflect.DeepEqual(layer, expected) {
		t.Errorf("layer_with_many_params did not match expected. Got: %+v", layer)
	}
}

func TestLayerWithNoExp(t *testing.T) {
	result := setupStatsigGCIR(t, "layer_configs", []string{"test_layer_with_no_exp"}, statsig.NONE)

	layers := result["layer_configs"].(map[string]interface{})
	layer := layers["test_layer_with_no_exp"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                            "test_layer_with_no_exp",
		"value":                           map[string]interface{}{"a_param": "foo"},
		"id_type":                         "userID",
		"rule_id":                         "default",
		"group":                           "default",
		"is_device_based":                 false,
		"explicit_parameters":             []interface{}{},
		"secondary_exposures":             []interface{}{},
		"undelegated_secondary_exposures": []interface{}{},
	}

	if !reflect.DeepEqual(layer, expected) {
		t.Errorf("test_layer_with_no_exp did not match expected. Got: %+v", layer)
	}
}

func TestAutotune(t *testing.T) {
	result := setupStatsigGCIR(t, "dynamic_configs", []string{"test_autotune"}, statsig.NONE)

	dcs := result["dynamic_configs"].(map[string]interface{})
	exp := dcs["test_autotune"].(map[string]interface{})

	expected := map[string]interface{}{
		"name":                "test_autotune",
		"value":               map[string]interface{}{},
		"rule_id":             "5380HnrABE4p869fZhtUV9",
		"group":               "5380HnrABE4p869fZhtUV9",
		"group_name":          "black",
		"is_device_based":     false,
		"id_type":             "userID",
		"secondary_exposures": []interface{}{},
	}

	if !reflect.DeepEqual(exp, expected) {
		t.Errorf("test_autotune did not match expected. Got: %+v", exp)
	}
}

func TestLength(t *testing.T) {

	gcir_result := setupStatsigGCIR(t, "", nil, statsig.NONE)

	fg_result := gcir_result["feature_gates"].(map[string]interface{})
	dc_result := gcir_result["dynamic_configs"].(map[string]interface{})
	l_result := gcir_result["layer_configs"].(map[string]interface{})

	if len(fg_result) != 65 {
		t.Errorf("expected 65 feature gates, but got %d", len(fg_result))
	}
	if len(dc_result) != 62 {
		t.Errorf("expected 62 dynamic configs, but got %d", len(dc_result))
	}
	if len(l_result) != 12 {
		t.Errorf("expected 12 layers, but got %d", len(l_result))
	}
}
