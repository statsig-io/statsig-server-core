package tests

import (
	"testing"
)

func TestLogEvent(t *testing.T) {
	user, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", nil)

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
