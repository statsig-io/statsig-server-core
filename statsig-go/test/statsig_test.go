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
