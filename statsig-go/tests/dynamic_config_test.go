package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func CreateDynamicConfigOptions(scrapiServer *MockScrapi) *statsig.StatsigOptions {
	return statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(scrapiServer.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithOutputLogLevel("DEBUG").
		Build()
}

func TestEnvironmentConfig(t *testing.T) {
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: false}

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
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: false}

	dynamic_config := "test_environment_config"
	_ = s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged while disable exposure logging was set to false")
	}

}

func TestDynamicConfigDisableExposureLoggingIsTrue(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: true}
	_, _ = s.Initialize()

	dynamic_config := "test_environment_config"
	_ = s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)

	s.Shutdown()

	if checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was logged while disable exposure logging was set to true")
	}

}

func TestDynamicConfiExposureLoggingNoOptions(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	dynamic_config := "test_environment_config"
	_ = s.GetDynamicConfig(*user, dynamic_config, nil)

	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged when options were not supplied")
	}

}

func TestConfigManualLogging(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	dynamicConfigOptions := &statsig.GetDynamicConfigOptions{DisableExposureLogging: true}
	_, _ = s.Initialize()

	dynamic_config := "test_environment_config"
	_ = s.GetDynamicConfig(*user, dynamic_config, dynamicConfigOptions)
	s.ManuallyLogDynamicConfigExposure(*user, dynamic_config)

	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged")
	}
}
