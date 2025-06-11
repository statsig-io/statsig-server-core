package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestStatsigInitializeWithDetails(t *testing.T) {

	scrapiServer := serverSetup("eval_proj_dcs.json")

	o := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl(scrapiServer.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithEnvironment("production").
		WithEventLoggingFlushIntervalMs(2000).
		WithEventLoggingMaxQueueSize(5000).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		Build()

	s, teardown := statsigSetup(t, o)
	defer teardown()
	res, _ := s.InitializeWithDetails()

	if !res.IsConfigSpecReady {
		t.Errorf("expected IsConfigSpecReady to be true, got %v", res.IsConfigSpecReady)
	}

	if res.IsIdListReady != nil {
		t.Errorf("expected IsIdListReady to be nil, got %v", res.IsIdListReady)
	}

	if !res.InitSuccess {
		t.Errorf("expected InitSuccess to be true, got %v", res.InitSuccess)
	}

	if res.Duration <= 0 {
		t.Errorf("expected Duration to be > 0, got %f", res.Duration)
	}

	if res.Source != "Network" {
		t.Errorf("expected Source to be 'Network', got '%s'", res.Source)
	}

	if res.FailureDetails != nil {
		t.Errorf("expected FailureDetails to be nil, got %v", res.FailureDetails)
	}

}

func TestStatsigInitializeWithDetailsFailure(t *testing.T) {

	scrapiServer := serverSetup("eval_proj_dcs.json")

	o := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("http://invalid.url").
		WithLogEventUrl(scrapiServer.GetUrlForEndpoint("/v1/log_event")).
		WithOutputLogLevel("DEBUG").
		Build()

	s, teardown := statsigSetup(t, o)
	defer teardown()
	res, _ := s.InitializeWithDetails()

	if res.IsConfigSpecReady {
		t.Errorf("expected IsConfigSpecReady to be false, got %v", res.IsConfigSpecReady)
	}

	if res.IsIdListReady != nil {
		t.Errorf("expected IsIdListReady to be nil, got %v", res.IsIdListReady)
	}

	if !res.InitSuccess {
		t.Errorf("expected InitSuccess to be true, got %v", res.InitSuccess)
	}

	if res.Duration < 0 {
		t.Errorf("expected Duration to be >= 0, got %f", res.Duration)
	}

	if res.Source != "NoValues" {
		t.Errorf("expected Source to be 'NoValues', got '%s'", res.Source)
	}

	if res.FailureDetails == nil {
		t.Errorf("expected FailureDetails to be not nil, got %v", res.FailureDetails)
	}

}
