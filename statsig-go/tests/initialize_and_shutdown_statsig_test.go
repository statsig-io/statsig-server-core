package tests

import (
	"testing"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

func TestStatsigInitialize(t *testing.T) {

	scrapiServer := serverSetup("eval_proj_dcs.json", nil)

	o := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(scrapiServer.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithEnvironment("production").
		WithEventLoggingFlushIntervalMs(2000).
		WithEventLoggingMaxQueueSize(5000).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		Build()

	_, _, _, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "a-user", o)

	defer teardown()

}
