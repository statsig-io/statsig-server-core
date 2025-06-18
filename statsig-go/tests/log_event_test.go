package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestLogEvent(t *testing.T) {

	user := statsig.NewStatsigUserBuilder().
		WithUserID("a-user").Build()

	scrapiServer := serverSetup("")

	options := CreateFeatureGateOptions(scrapiServer)

	s, teardown := statsigSetup(t, options)
	defer teardown()

	event := map[string]interface{}{
		"name":  "sample event",
		"value": "event",
		"metadata": map[string]string{
			"val_1": "testing log event",
			"val_2": "blah blah",
			"val_3": "thing",
		},
	}

	s.LogEvent(*user, event)
	s.FlushEvents()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "sample event") {
		t.Errorf("Error occurred, sample event was not logged")
	}
}
