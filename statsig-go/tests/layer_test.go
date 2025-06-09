package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func CreateLayerStatsigOptions(scrapiServer *MockScrapi) *statsig.StatsigOptions {
	return statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(scrapiServer.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithOutputLogLevel("DEBUG").
		Build()
}

func TestBasicLayer(t *testing.T) {

	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateLayerStatsigOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	if layer.Name != "test_layer" {
		t.Errorf("expected Name to be 'test_layer', got '%v'", layer.Name)
	}

	if layer.Value["another_param"] != false {
		t.Errorf("expected Value['another_param'] to be false, got %v", layer.Value["another_param"])
	}

	if layer.Value["layer_param"] != "test" {
		t.Errorf("expected Value['layer_param'] to be test, got %v", layer.Value["layer_param"])
	}

	if layer.RuleID != "1TKp6f9jRaorF9zzEgTjkH" {
		t.Errorf("expected RuleID to be 1TKp6f9jRaorF9zzEgTjkH, got '%v'", layer.RuleID)
	}

	if layer.IdType != "userID" {
		t.Errorf("expected IdType to be userID, got '%v'", layer.IdType)
	}

	if layer.EvaluationDetails.Reason != "Network:Recognized" {
		t.Errorf("expected Reason to be 'Network:Recognized', got '%v'", layer.EvaluationDetails.Reason)
	}

	if layer.EvaluationDetails.Lcut == 0 {
		t.Errorf("expected Lcut to be a non-zero int, got '%v'", layer.EvaluationDetails.Lcut)
	}

	if layer.EvaluationDetails.ReceivedAt == 0 {
		t.Errorf("expected ReceivedAt to be a non-zero int, got '%v'", layer.EvaluationDetails.ReceivedAt)
	}

	if layer.AllocatedExperimentName != "test_exp_with_layers_and_holdout" {
		t.Errorf("expected AllocatedExperimentName to be test_exp_with_layers_and_holdout, got '%v'", layer.AllocatedExperimentName)
	}

}

func TestLayerDisableExposureLoggingIsFalse(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateLayerStatsigOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	_ = layer.Get(user, "another_param")
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was not logged while disable exposure logging was set to false")
	}

}

func TestLayerDisableExposureLoggingIsTrue(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateLayerStatsigOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: true}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	_ = layer.Get(user, "another_param")
	s.Shutdown()

	if checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was logged while disable exposure logging was set to true")
	}

}

func TestGetNonexistentParamInLayerWithExposureLogging(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateLayerStatsigOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	_ = layer.Get(user, "dummy_param")
	s.Shutdown()

	if checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was logged while trying to get param that does not exist")
	}

}

func TestGetRetrievesCorrectParamValue(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateLayerStatsigOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	val := layer.Get(user, "another_param")

	if val != false {
		t.Errorf("expected Value['another_param'] to be false, got %v", val)
	}

}

func TestExposureLoggingOccursWithNoOptions(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateLayerStatsigOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, nil)

	_ = layer.Get(user, "another_param")
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was not logged while disable exposure logging was set to false")
	}

}
