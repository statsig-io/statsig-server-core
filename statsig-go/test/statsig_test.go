package test

import (
	"os"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestCheckGateEvaluation(t *testing.T) {
	statsig, _, user := setupStatsig(t)

	gate := statsig.CheckGate(user, "test_public")
	if !gate {
		t.Errorf("Gate test_public is false but should be true")
	}

	statsig.Shutdown()
}

func TestFeatureGateEvaluation(t *testing.T) {
	statsig, _, user := setupStatsig(t)

	gate := statsig.GetFeatureGate(user, "test_public")
	if !gate.Value {
		t.Errorf("Gate test_public is false but should be true")
	}

	statsig.Shutdown()

}

func TestDynamicConfigEvaluation(t *testing.T) {
	statsig, _, user := setupStatsig(t)

	config := statsig.GetDynamicConfig(user, "test_email_config")
	result := config.GetString("header_text", "err")
	if result != "everyone else" {
		t.Errorf("Dynamic config header_text is not correct, got '%s'", result)
	}

	statsig.Shutdown()
}

func TestExperimentEvaluation(t *testing.T) {
	statsig, _, user := setupStatsig(t)

	experiment := statsig.GetExperiment(user, "exp_with_obj_and_array")
	result := experiment.GetMap("obj_param", map[string]any{})
	if result["group"] != "test" {
		t.Errorf("Experiment obj_param is not correct, got %v", result)
	}

	statsig.Shutdown()
}

func TestLayerEvaluation(t *testing.T) {
	statsig, _, user := setupStatsig(t)

	layer := statsig.GetLayer(user, "layer_with_many_params")
	result := layer.GetString("a_string", "err")
	if result != "layer" {
		t.Errorf("Layer a_string is not correct, got '%s'", result)
	}

	statsig.Shutdown()
}

func TestEventLogging(t *testing.T) {
	statsig, scrapi, user := setupStatsig(t)

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

func setupStatsig(t *testing.T) (*statsig_go.Statsig, *MockScrapi, *statsig_go.StatsigUser) {
	scrapi := NewMockScrapi()

	resData, err := os.ReadFile("../../statsig-rust/tests/data/eval_proj_dcs.json")
	if err != nil {
		t.Errorf("error reading file: %v", err)
	}

	scrapi.Stub("GET", "/v2/download_config_specs/secret-123.json", StubResponse{
		Status: 200,
		Body:   resData,
	})

	scrapi.Stub("POST", "/v1/log_event", StubResponse{
		Status: 200,
		Body:   []byte(`{"success": true}`),
	})

	opts, err := statsig_go.NewOptionsBuilder().
		WithSpecsUrl(scrapi.URL() + "/v2/download_config_specs").
		WithLogEventUrl(scrapi.URL() + "/v1/log_event").
		Build()

	if err != nil {
		t.Errorf("error creating StatsigOptions: %v", err)
	}

	user, err := statsig_go.NewUserBuilderWithUserID("user-id").Build()
	if err != nil {
		t.Errorf("error creating StatsigUser: %v", err)
	}

	statsig := statsig_go.NewStatsigWithOptions("secret-123", opts)
	statsig.Initialize()

	return statsig, scrapi, user
}
