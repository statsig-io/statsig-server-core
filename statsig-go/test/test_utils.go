package test

import (
	"os"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func SetupTest(t *testing.T) (*statsig_go.Statsig, *MockScrapi, *statsig_go.StatsigUser) {
	resData, err := os.ReadFile("../../statsig-rust/tests/data/eval_proj_dcs.json")
	if err != nil {
		t.Errorf("error reading file: %v", err)
	}

	return SetupTestWithDcsData(t, resData)
}

func SetupTestWithDcsData(t *testing.T, dcsData []byte) (*statsig_go.Statsig, *MockScrapi, *statsig_go.StatsigUser) {
	scrapi := NewMockScrapi()

	scrapi.Stub("GET", "/v2/download_config_specs/secret-123.json", StubResponse{
		Status: 200,
		Body:   dcsData,
	})

	scrapi.Stub("POST", "/v1/log_event", StubResponse{
		Status: 200,
		Body:   []byte(`{"success": true}`),
	})

	statsig, user := SetupStatsig(t, scrapi)

	statsig.Initialize()

	return statsig, scrapi, user
}

func SetupStatsig(t *testing.T, scrapi *MockScrapi) (*statsig_go.Statsig, *statsig_go.StatsigUser) {
	opts, err := statsig_go.NewOptionsBuilder().
		WithSpecsUrl(scrapi.URL() + "/v2/download_config_specs").
		WithLogEventUrl(scrapi.URL() + "/v1/log_event").
		WithEventLoggingMaxQueueSize(10).
		Build()

	if err != nil {
		t.Errorf("error creating StatsigOptions: %v", err)
	}

	user, err := statsig_go.NewUserBuilderWithUserID("user-id").Build()
	if err != nil {
		t.Errorf("error creating StatsigUser: %v", err)
	}

	statsig, err := statsig_go.NewStatsigWithOptions("secret-123", opts)
	if err != nil {
		t.Errorf("error creating Statsig: %v", err)
	}

	return statsig, user
}
