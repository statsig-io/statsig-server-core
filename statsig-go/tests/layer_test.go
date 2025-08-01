package tests

import (
	"testing"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

func TestBasicLayer(t *testing.T) {
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

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
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

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
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

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

func TestLayerLogLayerParamExposure(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	layerOptions := &statsig.GetLayerOptions{}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	_ = layer.Get(user, "another_param")
	s.Shutdown()

	events := scrapiServer.fetchLoggedEvents()
	if !checkEventNameExists(events, "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was not logged while disable exposure logging was set to true")
	}

	for _, event := range events {
		if event.EventName == "statsig::layer_exposure" {
			if event.Metadata["isManualExposure"] == "true" {
				t.Errorf("Error occurred, layer exposure event should be logged without manual exposure set to true")
			}
		}
	}

}

func TestGetNonexistentParamInLayerWithExposureLogging(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

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
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: false}

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	val := layer.Get(user, "another_param")

	if val != false {
		t.Errorf("expected Value['another_param'] to be false, got %v", val)
	}

}

func TestLayerExposureLoggingOccursNoOptions(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	layer_name := "test_layer"
	layer := s.GetLayer(*user, layer_name, nil)

	_ = layer.Get(user, "another_param")
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was not logged while options were not supplied")
	}

}

func TestLayerManualLogging(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	layerOptions := &statsig.GetLayerOptions{DisableExposureLogging: true}

	layer_name := "test_layer"
	param_name := "another_param"
	layer := s.GetLayer(*user, layer_name, layerOptions)

	_ = layer.Get(user, param_name)
	s.ManuallyLogLayerParameterExposure(*user, layer_name, param_name)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::layer_exposure") {
		t.Errorf("Error occurred, layer exposure event was not logged")
	}

}
