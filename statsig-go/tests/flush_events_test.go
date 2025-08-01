package tests

import (
	"testing"

	statsig "github.com/statsig-io/private-statsig-server-core/statsig-go/src"
)

func TestFlushEvents(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)
	defer teardown()

	checkGateOptions := &statsig.CheckGateOptions{DisableExposureLogging: false}

	feature_gate := "test_country_partial"
	_ = s.CheckGate(*user, feature_gate, checkGateOptions)

	if checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::gate_exposure") {
		t.Errorf("Error occurred, gate exposure event was logged before events were flushed")
	}

	s.FlushEvents()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::gate_exposure") {
		t.Errorf("Error occurred, gate exposure event was not logged after events were flushed")
	}

}
