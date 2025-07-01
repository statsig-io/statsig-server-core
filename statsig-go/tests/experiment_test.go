package tests

import (
	"reflect"
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func CreateExperimentOptions(scrapiServer *MockScrapi) *statsig.StatsigOptions {
	return statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(scrapiServer.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithOutputLogLevel("DEBUG").
		Build()
}

func TestExperimentBasic(t *testing.T) {

	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	experimentOptions := &statsig.GetExperimentOptions{DisableExposureLogging: false}

	experimentName := "exp_with_obj_and_array"
	experiment := s.GetExperiment(*user, experimentName, experimentOptions)

	if experiment.Name != "exp_with_obj_and_array" {
		t.Errorf("expected Name to be 'exp_with_obj_and_array', got '%v'", experiment.Name)
	}

	expectedArr := []interface{}{true, false, true}
	actualArr, ok := experiment.Value["arr_param"].([]interface{})
	if !ok {
		t.Errorf("expected arr_param to be []interface{}, got %T", experiment.Value["arr_param"])
	} else if !reflect.DeepEqual(actualArr, expectedArr) {
		t.Errorf("expected arr_param to be %v, got %v", expectedArr, actualArr)
	}

	expectedObj := map[string]interface{}{"group": "test"}
	actualObj, ok := experiment.Value["obj_param"].(map[string]interface{})
	if !ok {
		t.Errorf("expected obj_param to be map[string]interface{}, got %T", experiment.Value["obj_param"])
	} else if !reflect.DeepEqual(actualObj, expectedObj) {
		t.Errorf("expected obj_param to be %v, got %v", expectedObj, actualObj)
	}

	if experiment.RuleID != "23gt15KsgEAbUiwEapclqk" {
		t.Errorf("expected RuleID to be 23gt15KsgEAbUiwEapclqk, got '%v'", experiment.RuleID)
	}

	if experiment.IdType != "userID" {
		t.Errorf("expected IdType to be userID, got '%v'", experiment.IdType)
	}

	if experiment.EvaluationDetails.Reason != "Network:Recognized" {
		t.Errorf("expected Reason to be 'Network:Recognized', got '%v'", experiment.EvaluationDetails.Reason)
	}

	if experiment.EvaluationDetails.Lcut == 0 {
		t.Errorf("expected Lcut to be a non-zero int, got '%v'", experiment.EvaluationDetails.Lcut)
	}

	if experiment.EvaluationDetails.ReceivedAt == 0 {
		t.Errorf("expected ReceivedAt to be a non-zero int, got '%v'", experiment.EvaluationDetails.ReceivedAt)
	}

	if experiment.SecondaryExposures[0].Gate != "global_holdout" {
		t.Errorf("expected gate to be 'global_holdout', got '%v'", experiment.SecondaryExposures[0].Gate)
	}

	if experiment.SecondaryExposures[0].GateValue != "false" {
		t.Errorf("expected gateValue to be 'false', got '%v'", experiment.SecondaryExposures[0].GateValue)
	}

	if experiment.SecondaryExposures[0].RuleID != "3QoA4ncNdVGBaMt3N1KYjz:0.50:1" {
		t.Errorf("expected ruleID to be '3QoA4ncNdVGBaMt3N1KYjz:0.50:1', got '%v'", experiment.SecondaryExposures[0].RuleID)
	}
}

func TestExperimentDisableExposureLoggingIsFalse(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	experimentOptions := &statsig.GetExperimentOptions{DisableExposureLogging: false}

	experimentName := "exp_with_obj_and_array"
	_ = s.GetExperiment(*user, experimentName, experimentOptions)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged while disable exposure logging was set to false")
	}

}

func TestExperimentDisableExposureLoggingIsTrue(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	experimentOptions := &statsig.GetExperimentOptions{DisableExposureLogging: true}

	experimentName := "exp_with_obj_and_array"
	_ = s.GetExperiment(*user, experimentName, experimentOptions)
	s.Shutdown()

	if checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was logged while disable exposure logging was set to true")
	}

}

func TestExperimentExposureLoggingNoOptions(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	experimentName := "exp_with_obj_and_array"
	_ = s.GetExperiment(*user, experimentName, nil)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged when options were not supplied")
	}

}

func TestExperimentManualLogging(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateFeatureGateOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	experimentOptions := &statsig.GetExperimentOptions{DisableExposureLogging: true}

	experimentName := "exp_with_obj_and_array"
	_ = s.GetExperiment(*user, experimentName, experimentOptions)

	s.ManuallyLogExperimentExposure(*user, experimentName)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged.")
	}
}
