package test

import (
	"encoding/json"
	"strconv"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestCheckGateEvaluation(t *testing.T) {
	statsig, _, user := SetupTest(t)

	gate := statsig.CheckGate(user, "test_public")
	if !gate {
		t.Errorf("Gate test_public is false but should be true")
	}

	statsig.Shutdown()
}

func TestFeatureGateEvaluation(t *testing.T) {
	statsig, _, user := SetupTest(t)

	gate := statsig.GetFeatureGate(user, "test_public")
	if !gate.Value {
		t.Errorf("Gate test_public is false but should be true")
	}

	statsig.Shutdown()

}

func TestDynamicConfigEvaluation(t *testing.T) {
	statsig, _, user := SetupTest(t)

	config := statsig.GetDynamicConfig(user, "test_email_config")
	result := config.GetString("header_text", "err")
	if result != "everyone else" {
		t.Errorf("Dynamic config header_text is not correct, got '%s'", result)
	}

	statsig.Shutdown()
}

func TestExperimentEvaluation(t *testing.T) {
	statsig, _, user := SetupTest(t)

	experiment := statsig.GetExperiment(user, "exp_with_obj_and_array")
	result := experiment.GetMap("obj_param", map[string]any{})
	if result["group"] != "test" {
		t.Errorf("Experiment obj_param is not correct, got %v", result)
	}

	statsig.Shutdown()
}

func TestGetExperimentByGroupName(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	control := statsig.GetExperimentByGroupName("test_experiment_no_targeting", "Control")
	if control.GroupName == nil || *control.GroupName != "Control" {
		t.Errorf("Expected group name 'Control', got %v", control.GroupName)
	}
	if control.RuleID != "54QJztEPRLXK7ZCvXeY9q4" {
		t.Errorf("Expected rule ID '54QJztEPRLXK7ZCvXeY9q4', got '%s'", control.RuleID)
	}
	if control.IDType != "userID" {
		t.Errorf("Expected id type 'userID', got '%s'", control.IDType)
	}
	if v := control.GetString("value", "err"); v != "control" {
		t.Errorf("Expected value 'control', got '%s'", v)
	}

	test := statsig.GetExperimentByGroupName("test_experiment_no_targeting", "Test")
	if test.GroupName == nil || *test.GroupName != "Test" {
		t.Errorf("Expected group name 'Test', got %v", test.GroupName)
	}
	if v := test.GetString("value", "err"); v != "test_1" {
		t.Errorf("Expected value 'test_1', got '%s'", v)
	}
}

func TestGetExperimentByGroupNameNotFound(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	unknownExp := statsig.GetExperimentByGroupName("not_an_experiment", "Control")
	if unknownExp.GroupName != nil {
		t.Errorf("Expected nil group name for unknown experiment, got %v", *unknownExp.GroupName)
	}
	if unknownExp.RuleID != "" {
		t.Errorf("Expected empty rule ID for unknown experiment, got '%s'", unknownExp.RuleID)
	}

	unknownGroup := statsig.GetExperimentByGroupName("test_experiment_no_targeting", "InvalidGroupName")
	if unknownGroup.GroupName != nil {
		t.Errorf("Expected nil group name for unknown group, got %v", *unknownGroup.GroupName)
	}
	if unknownGroup.RuleID != "" {
		t.Errorf("Expected empty rule ID for unknown group, got '%s'", unknownGroup.RuleID)
	}
}

func TestGetExperimentByGroupIDAdvanced(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	experiment := statsig.GetExperimentByGroupIDAdvanced("test_experiment_no_targeting", "54QJztEPRLXK7ZCvXeY9q4")
	if experiment.GroupName == nil || *experiment.GroupName != "Control" {
		t.Errorf("Expected group name 'Control', got %v", experiment.GroupName)
	}
	if experiment.RuleID != "54QJztEPRLXK7ZCvXeY9q4" {
		t.Errorf("Expected rule ID '54QJztEPRLXK7ZCvXeY9q4', got '%s'", experiment.RuleID)
	}
	if experiment.IDType != "userID" {
		t.Errorf("Expected id type 'userID', got '%s'", experiment.IDType)
	}
	if v := experiment.GetString("value", "err"); v != "control" {
		t.Errorf("Expected value 'control', got '%s'", v)
	}
}

func TestOverrideExperimentByGroupNameGlobal(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	// Pick a target group whose value differs from the user's natural bucketing
	// so the assertion proves the override actually changed the result.
	baseline := statsig.GetExperiment(user, "test_experiment_no_targeting")
	baselineValue := baseline.GetString("value", "")
	targetGroup, targetValue := "Test", "test_1"
	if baselineValue == "test_1" {
		targetGroup, targetValue = "Control", "control"
	}

	statsig.OverrideExperimentByGroupName("test_experiment_no_targeting", targetGroup, nil)

	experiment := statsig.GetExperiment(user, "test_experiment_no_targeting")
	if v := experiment.GetString("value", ""); v != targetValue {
		t.Errorf("Expected overridden value '%s' for group '%s', got '%s'", targetValue, targetGroup, v)
	}
	if experiment.EvaluationDetails.Reason != "LocalOverride:Recognized" {
		t.Errorf("Expected reason 'LocalOverride:Recognized', got '%s'", experiment.EvaluationDetails.Reason)
	}
}

func TestOverrideExperimentByGroupNameForID(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	otherUser, err := statsig_go.NewUserBuilderWithUserID("other-user").Build()
	if err != nil {
		t.Fatalf("error creating StatsigUser: %v", err)
	}

	// Capture both users' natural bucketing before applying any override.
	baseline := statsig.GetExperiment(user, "test_experiment_no_targeting")
	baselineValue := baseline.GetString("value", "")
	otherBaseline := statsig.GetExperiment(otherUser, "test_experiment_no_targeting")
	otherBaselineValue := otherBaseline.GetString("value", "")

	targetGroup, targetValue := "Test", "test_1"
	if baselineValue == "test_1" {
		targetGroup, targetValue = "Control", "control"
	}

	userID := "user-id" // matches the user built by SetupTest
	statsig.OverrideExperimentByGroupName("test_experiment_no_targeting", targetGroup, &userID)

	experiment := statsig.GetExperiment(user, "test_experiment_no_targeting")
	if v := experiment.GetString("value", ""); v != targetValue {
		t.Errorf("Expected overridden value '%s' for id '%s', got '%s'", targetValue, userID, v)
	}

	// A different id must keep its natural bucketing — the per-id override must not leak.
	otherAfter := statsig.GetExperiment(otherUser, "test_experiment_no_targeting")
	otherAfterValue := otherAfter.GetString("value", "")
	if otherAfterValue != otherBaselineValue {
		t.Errorf("Per-id override leaked to a different id: '%s' -> '%s'", otherBaselineValue, otherAfterValue)
	}
}

func TestLayerEvaluation(t *testing.T) {
	statsig, _, user := SetupTest(t)

	layer := statsig.GetLayer(user, "layer_with_many_params")
	result := layer.GetString("a_string", "err")
	if result != "layer" {
		t.Errorf("Layer a_string is not correct, got '%s'", result)
	}

	anotherUser, err := statsig_go.NewUserBuilderWithUserID("a_user").Build()
	if err != nil {
		t.Errorf("error creating StatsigUser: %v", err)
	}

	anotherLayer := statsig.GetLayer(anotherUser, "layer_with_many_params")
	result = anotherLayer.GetString("a_string", "err")
	if result != "test_2" {
		t.Errorf("Layer a_string is not correct, got '%s'", result)
	}

	if anotherLayer.AllocatedExperimentName == nil || *anotherLayer.AllocatedExperimentName != "experiment_with_many_params" {
		t.Errorf("Layer allocated experiment name is not correct, got '%s'", *anotherLayer.AllocatedExperimentName)
	}

	statsig.Shutdown()
}

func TestEventLogging(t *testing.T) {
	statsig, scrapi, user := SetupTest(t)

	statsig.LogEvent(user, statsig_go.EventPayload{
		EventName: "test_event",
		Value:     1,
		Metadata: map[string]string{
			"test_key": "test_value",
		},
	})

	statsig.Shutdown()

	events := scrapi.Events()
	if len(events) == 0 {
		t.Errorf("No events found")
	}

	event := events[0]
	if event["eventName"] != "test_event" {
		t.Errorf("Event name is not correct")
	}
}

func TestGetClientInitResponse(t *testing.T) {
	statsig, _, user := SetupTest(t)

	hashAlgo := "none"
	options := statsig_go.ClientInitResponseOptions{
		HashAlgorithm: &hashAlgo,
	}

	rawResponse := statsig.GetClientInitResponseWithOptions(user, &options)
	statsig.Shutdown()

	if rawResponse == nil {
		t.Errorf("Response is nil")
		return
	}

	response := map[string]any{}
	err := json.Unmarshal([]byte(*rawResponse), &response)
	if err != nil {
		t.Errorf("error unmarshalling client init response: %v", err)
	}

	if response["feature_gates"] == nil {
		t.Errorf("Feature gates is nil")
	}

	gates := response["feature_gates"].(map[string]any)
	gate := gates["test_public"]
	if gate == nil {
		t.Errorf("Test public gate is nil")
	}
}

func TestGetClientInitResponseNoOptions(t *testing.T) {
	statsig, _, user := SetupTest(t)

	rawResponse := statsig.GetClientInitResponseWithOptions(user, nil)
	statsig.Shutdown()

	if rawResponse == nil {
		t.Errorf("Response is nil")
		return
	}

	response := map[string]any{}
	err := json.Unmarshal([]byte(*rawResponse), &response)
	if err != nil {
		t.Errorf("error unmarshalling client init response: %v", err)
	}

	if response["feature_gates"] == nil {
		t.Errorf("Feature gates is nil")
	}

	gates := response["feature_gates"].(map[string]any)
	if len(gates) == 0 {
		t.Errorf("Feature gates is empty")
	}
}

func TestShutdownCycling(t *testing.T) {
	_, scrapi, _ := SetupTest(t)

	for range 25 {
		statsig, _ := SetupStatsig(t, scrapi)
		statsig.Initialize()

		for i := range 100 {
			user, _ := statsig_go.NewUserBuilderWithUserID("user-id-" + strconv.Itoa(i)).Build()
			statsig.CheckGate(user, "test_public")
		}

		statsig.Shutdown()
	}
}
