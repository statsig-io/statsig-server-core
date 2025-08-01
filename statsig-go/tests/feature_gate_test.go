package tests

import (
	"testing"

	statsig "github.com/statsig-io/private-statsig-server-core/statsig-go/src"
)

func TestFeatureGateBasic(t *testing.T) {
	user, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)
	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}
	featureGate := s.GetFeatureGate(*user, "test_public", checkGateOptions)

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
		t.Errorf("expected Lcut to be non-zero, got '%v'", featureGate.EvaluationDetails.Lcut)
	}
	if featureGate.EvaluationDetails.ReceivedAt == 0 {
		t.Errorf("expected ReceivedAt to be non-zero, got '%v'", featureGate.EvaluationDetails.ReceivedAt)
	}
}

func TestDisableExposureLoggingIsFalse(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)
	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}
	_, _ = s.Initialize()

	_ = s.CheckGate(*user, "test_country_partial", checkGateOptions)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::gate_exposure") {
		t.Errorf("expected gate exposure event to be logged when DisableExposureLogging is false")
	}
}

func TestDisableExposureLoggingIsTrue(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)
	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: true}
	_ = s.CheckGate(*user, "test_country_partial", checkGateOptions)
	s.Shutdown()

	if checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::gate_exposure") {
		t.Errorf("expected gate exposure event NOT to be logged when DisableExposureLogging is true")
	}
}

func TestFeatureGateExposureLoggingNoOptions(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)
	defer teardown()

	_ = s.CheckGate(*user, "test_country_partial", nil)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::gate_exposure") {
		t.Errorf("expected gate exposure event to be logged when no options were supplied")
	}
}

func TestManualLoggingGate(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: true}

	feature_gate := "test_country_partial"
	_ = s.CheckGate(*user, feature_gate, checkGateOptions)

	s.ManuallyLogGateExposure(*user, feature_gate)
	s.Shutdown()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::gate_exposure") {
		t.Errorf("Error occurred, gate exposure event was not logged")
	}

}
