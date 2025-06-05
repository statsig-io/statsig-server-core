package tests

import (
	"os"
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func LoadTestData(path string) []byte {
	path_to_rust_data := "../../statsig-rust/tests/data/"
	data, _ := os.ReadFile(path_to_rust_data + path)
	return data
}

func serverSetup(dcs_path string) *MockScrapi {
	mock_scrapi := NewMockScrapiServer()
	dcs_content := LoadTestData(dcs_path)

	mock_scrapi.Mock("/v2/download_config_specs/secret-key.json", 200, dcs_content)
	mock_scrapi.Mock("/v1/log_event", 200, []byte(`{"success": true}`))

	return mock_scrapi
}

func statsigSetup(t *testing.T, options *statsig.StatsigOptions) (statsig.Statsig, func()) {

	statsig, _ := statsig.NewStatsig("secret-key", *options)

	_, err := statsig.Initialize()

	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}

	teardown := func() {
		statsig.Shutdown()
	}

	return *statsig, teardown
}

func checkEventNameExists(events []statsig.Event, eventName string) bool {
	for _, e := range events {
		if e.EventName == eventName {
			return true
		}
	}
	return false
}
