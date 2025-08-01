package tests

import (
	"fmt"
	"os"
	"testing"

	statsig "github.com/statsig-io/private-statsig-server-core/statsig-go/src"
)

func LoadTestData(path string) []byte {
	path_to_rust_data := "../../statsig-rust/tests/data/"
	data, err := os.ReadFile(path_to_rust_data + path)
	if err != nil {
		fmt.Printf("Failed to load test data: %v", err)
	}
	return data
}

func serverSetup(dcs_path string) *MockScrapi {
	mock_scrapi := NewMockScrapiServer()
	dcs_content := LoadTestData(dcs_path)

	mock_scrapi.Mock("/v2/download_config_specs/secret-key.json", 200, dcs_content)
	mock_scrapi.Mock("/v1/log_event", 200, []byte(`{"success": true}`))

	return mock_scrapi
}

func setupStatsigTest(
	t *testing.T,
	jsonFile string,
	userId string,
	statsigOptions *statsig.StatsigOptions,
) (*statsig.StatsigUser, *MockScrapi, *statsig.Statsig, func()) {
	t.Helper()

	user := statsig.NewStatsigUserBuilder().
		WithUserID(userId).Build()

	scrapi := serverSetup(jsonFile)

	var options statsig.StatsigOptions

	if statsigOptions != nil {
		options = *statsigOptions
	} else {
		options = *statsig.NewStatsigOptionsBuilder().
			WithSpecsUrl(scrapi.GetUrlForEndpoint("/v2/download_config_specs")).
			WithLogEventUrl(scrapi.GetUrlForEndpoint("/v1/log_event")).
			WithOutputLogLevel("DEBUG").
			Build()
	}

	statsig, _ := statsig.NewStatsig("secret-key", options)

	_, err := statsig.Initialize()

	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}

	teardown := func() {
		statsig.Shutdown()
	}

	return user, scrapi, statsig, teardown
}

func checkEventNameExists(events []statsig.Event, eventName string) bool {
	for _, e := range events {
		if e.EventName == eventName {
			return true
		}
	}
	return false
}
