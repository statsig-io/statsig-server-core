package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestStatsigInitialize(t *testing.T) {

	scrapiServer := serverSetup("eval_proj_dcs.json")

	o := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(scrapiServer.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithSpecsAdapterRef(12345).
		WithEventLoggingAdapterRef(12345).
		WithEnvironment("production").
		WithEventLoggingFlushIntervalMs(2000).
		WithEventLoggingMaxQueueSize(5000).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		Build()

	_, teardown := statsigSetup(t, o)

	defer teardown()

}
