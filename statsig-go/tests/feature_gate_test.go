package tests

import (
	"net/http/httptest"
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func CreateGateTestOptions(server *httptest.Server) *statsig.StatsigOptions {
	return statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(server.URL + "/v2/download_config_specs").
		WithLogEventUrl(server.URL + "/v1/log_event").
		WithOutputLogLevel("DEBUG").
		Build()
}

func TestFeatureGateBasic(t *testing.T) {

	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	events := []statsig.Event{}

	server := setupServer(testServerOptions{
		onLogEvent: func(newEvents []map[string]interface{}) {
			for _, e := range newEvents {
				events = append(events, convertToExposureEvent(e))
			}
		},
	})
	defer server.Close()
	options := CreateGateTestOptions(server)

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}
	s, _ := statsig.NewStatsig("secret-key", *options)
	s.Initialize()

	feature_gate := "test_public"
	featureGate := s.GetFeatureGate(*user, feature_gate, checkGateOptions)

	s.Shutdown()

	if featureGate.Name != "test_public" {
		t.Errorf("expected Name to be 'test_public', got '%v'", featureGate.Name)
	}

	if featureGate.Value != true {
		t.Errorf("expected Value to be true, got %v", featureGate.Value)
	}

	if featureGate.RuleID != "6X3qJgyfwA81IJ2dxI7lYp" {
		t.Errorf("expected RuleID to be 6X3qJgyfwA81IJ2dxI7lYp, got '%v'", featureGate.RuleID)
	}

	if featureGate.IdType != "userID" {
		t.Errorf("expected IdType to be userID, got '%v'", featureGate.IdType)
	}

	if featureGate.EvaluationDetails.Reason != "Network:Recognized" {
		t.Errorf("expected Reason to be 'Network:Recognized', got '%v'", featureGate.EvaluationDetails.Reason)
	}

	if featureGate.EvaluationDetails.Lcut == 0 {
		t.Errorf("expected Lcut to be a non-zero int, got '%v'", featureGate.EvaluationDetails.Lcut)
	}

	if featureGate.EvaluationDetails.ReceivedAt == 0 {
		t.Errorf("expected ReceivedAt to be a non-zero int, got '%v'", featureGate.EvaluationDetails.ReceivedAt)
	}

}

func TestDisableExposureLoggingIsFalse(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()
	events := []statsig.Event{}

	server := setupServer(testServerOptions{
		onLogEvent: func(newEvents []map[string]interface{}) {
			for _, e := range newEvents {
				events = append(events, convertToExposureEvent(e))
			}
		},
	})
	defer server.Close()

	options := CreateGateTestOptions(server)

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}

	s, _ := statsig.NewStatsig("secret-key", *options)
	s.Initialize()

	feature_gate := "test_country_partial"
	_ = s.CheckGate(*user, feature_gate, checkGateOptions)
	s.Shutdown()

	if !checkEventNameExists(events, "statsig::gate_exposure") {
		t.Errorf("Error occurred, gate exposure event was not logged while disable exposure logging was set to false")
	}

}

func TestDisableExposureLoggingIsTrue(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	events := []statsig.Event{}

	server := setupServer(testServerOptions{
		onLogEvent: func(newEvents []map[string]interface{}) {
			for _, e := range newEvents {
				events = append(events, convertToExposureEvent(e))
			}
		},
	})
	defer server.Close()

	options := CreateGateTestOptions(server)

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: true}

	s, _ := statsig.NewStatsig("secret-key", *options)
	s.Initialize()

	feature_gate := "test_country_partial"
	_ = s.CheckGate(*user, feature_gate, checkGateOptions)
	s.Shutdown()

	if checkEventNameExists(events, "statsig::gate_exposure") {
		t.Errorf("Error occurred, gate exposure event was logged while disable exposure logging was set to true")
	}

}
