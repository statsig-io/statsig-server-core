package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestFlushEvents(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("eval_proj_dcs.json")

	options := CreateFeatureGateOptions(scrapiServer)

	s, _ := statsigSetup(t, options)

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
