package tests

import (
	"net/http/httptest"
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func CreateDynamicConfigOptions(server *httptest.Server) *statsig.StatsigOptions {
	return statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(server.URL + "/v2/download_config_specs").
		WithLogEventUrl(server.URL + "/v1/log_event").
		WithOutputLogLevel("DEBUG").
		Build()
}

func TestEnvironmentConfig(t *testing.T) {

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
	options := CreateDynamicConfigOptions(server)

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: false}
	s, _ := statsig.NewStatsig("secret-key", *options)
	s.Initialize()

	dynamic_config := "test_environment_config"
	dynamicConfig := s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	s.Shutdown()

	if dynamicConfig.Name != "test_environment_config" {
		t.Errorf("expected Name to be 'test_environment_config', got '%v'", dynamicConfig.Name)
	}

	if dynamicConfig.Value["header_text"] != "production" {
		t.Errorf("expected Value['header_text'] to be production, got %v", dynamicConfig.Value["header_text"])
	}

	if dynamicConfig.RuleID != "default" {
		t.Errorf("expected RuleID to be default, got '%v'", dynamicConfig.RuleID)
	}

	if dynamicConfig.IdType != "userID" {
		t.Errorf("expected IdType to be userID, got '%v'", dynamicConfig.IdType)
	}

	if dynamicConfig.EvaluationDetails.Reason != "Network:Recognized" {
		t.Errorf("expected Reason to be 'Network:Recognized', got '%v'", dynamicConfig.EvaluationDetails.Reason)
	}

	if dynamicConfig.EvaluationDetails.Lcut == 0 {
		t.Errorf("expected Lcut to be a non-zero int, got '%v'", dynamicConfig.EvaluationDetails.Lcut)
	}

	if dynamicConfig.EvaluationDetails.ReceivedAt == 0 {
		t.Errorf("expected ReceivedAt to be a non-zero int, got '%v'", dynamicConfig.EvaluationDetails.ReceivedAt)
	}

}

func TestDynamicConfigDisableExposureLoggingIsFalse(t *testing.T) {
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

	options := CreateDynamicConfigOptions(server)

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: false}
	s, _ := statsig.NewStatsig("secret-key", *options)
	s.Initialize()

	dynamic_config := "test_environment_config"
	_ = s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	s.Shutdown()

	if !checkEventNameExists(events, "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged while disable exposure logging was set to false")
	}

}

func TestDynamicConfigDisableExposureLoggingIsTrue(t *testing.T) {
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

	options := CreateDynamicConfigOptions(server)

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: true}
	s, _ := statsig.NewStatsig("secret-key", *options)
	s.Initialize()

	dynamic_config := "test_environment_config"
	_ = s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	s.Shutdown()

	if checkEventNameExists(events, "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was logged while disable exposure logging was set to true")
	}

}
