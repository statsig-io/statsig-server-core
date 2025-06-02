package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestStatsigShutdown(t *testing.T) {
	server := setupServer(testServerOptions{})

	o := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(server.URL + "/v2/download_config_specs").
		WithLogEventUrl(server.URL + "/v1/log_event").
		WithSpecsAdapterRef(12345).
		WithEventLoggingAdapterRef(12345).
		WithEnvironment("production").
		WithEventLoggingFlushIntervalMs(2000).
		WithEventLoggingMaxQueueSize(5000).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		Build()
	s, _ := statsig.NewStatsig("secret-key", *o)

	s.Initialize()

	s.Shutdown()

}
