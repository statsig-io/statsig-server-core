package tests

import (
	"testing"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestStatsigOptionsBasic(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("https://example.com/specs").
		WithLogEventUrl("https://example.com/log").
		WithSpecsAdapterRef(uint64(12345)).
		WithEventLoggingAdapterRef(uint64(12345)).
		WithEnvironment("production").
		WithEventLoggingFlushIntervalMs(2000).
		WithEventLoggingMaxQueueSize(5000).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		WithDisableCountryLookup(true).
		WithDisableUserAgentParsing(true).
		WithWaitForCountryLookupInit(false).
		Build()

	if *options.SpecsUrl != "https://example.com/specs" {
		t.Errorf("expected SpecsUrl to be 'https://example.com/specs', got %v", options.SpecsUrl)
	}
	if *options.LogEventUrl != "https://example.com/log" {
		t.Errorf("expected LogEventUrl to be 'https://example.com/log', got %v", options.LogEventUrl)
	}
	if options.SpecsAdapterRef != uint64(12345) {
		t.Errorf("expected SpecsAdapterRef to be 'adapter_specs', got %v", options.SpecsAdapterRef)
	}
	if options.EventLoggingAdapterRef != uint64(12345) {
		t.Errorf("expected EventLoggingAdapterRef to be 'adapter_events', got %v", options.EventLoggingAdapterRef)
	}
	if *options.Environment != "production" {
		t.Errorf("expected Environment to be 'production', got %v", options.Environment)
	}
	if options.EventLoggingFlushIntervalMs != 2000 {
		t.Errorf("expected EventLoggingFlushIntervalMs to be 2000, got %d", options.EventLoggingFlushIntervalMs)
	}
	if options.EventLoggingMaxQueueSize != 5000 {
		t.Errorf("expected EventLoggingMaxQueueSize to be 5000, got %d", options.EventLoggingMaxQueueSize)
	}
	if options.SpecsSyncIntervalMs != 1000 {
		t.Errorf("expected SpecsSyncIntervalMs to be 1000, got %d", options.SpecsSyncIntervalMs)
	}
	if *options.OutputLogLevel != "DEBUG" {
		t.Errorf("expected OutputLogLevel to be 'DEBUG', got %v", options.OutputLogLevel)
	}
	if *options.DisableCountryLookup != true {
		t.Errorf("expected DisableCountryLookup to be 'true', got %v", *options.DisableCountryLookup)
	}
	if *options.DisableUserAgentParsing != true {
		t.Errorf("expected DisableUserAgentParsing to be 'true', got %v", *options.DisableUserAgentParsing)
	}
	if *options.WaitForCountryLookupInit != false {
		t.Errorf("expected WaitForCountryLookupInit to be 'false', got %v", *options.WaitForCountryLookupInit)
	}
	if options.WaitForUserAgentInit != nil {
		t.Errorf("expected WaitForUserAgentInit to be 'nil', got %v", *options.WaitForUserAgentInit)
	}

}

func TestBuilderSetNumericValues(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsSyncIntervalMs(12345).
		WithEventLoggingFlushIntervalMs(67890).
		WithEventLoggingMaxQueueSize(99999).
		Build()

	if options.SpecsSyncIntervalMs != 12345 {
		t.Errorf("expected SpecsSyncIntervalMs to be 12345, got %d", options.SpecsSyncIntervalMs)
	}
	if options.EventLoggingFlushIntervalMs != 67890 {
		t.Errorf("expected EventLoggingFlushIntervalMs to be 67890, got %d", options.EventLoggingFlushIntervalMs)
	}
	if options.EventLoggingMaxQueueSize != 99999 {
		t.Errorf("expected EventLoggingMaxQueueSize to be 99999, got %d", options.EventLoggingMaxQueueSize)
	}
}

func TestBuilderSetStringValues(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("https://example.com/specs").
		WithLogEventUrl("").
		WithEnvironment("staging").
		WithOutputLogLevel("INFO").
		Build()

	if *options.SpecsUrl != "https://example.com/specs" {
		t.Errorf("unexpected SpecsUrl: %v", options.SpecsUrl)
	}
	if *options.LogEventUrl != "" {
		t.Errorf("expected empty LogEventUrl, got %v", options.LogEventUrl)
	}
	if *options.Environment != "staging" {
		t.Errorf("unexpected Environment: %v", options.Environment)
	}
	if *options.OutputLogLevel != "INFO" {
		t.Errorf("unexpected OutputLogLevel: %v", options.OutputLogLevel)
	}
}

func TestBuilderEmptyValues(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("").
		WithLogEventUrl("").
		WithEnvironment("").
		WithOutputLogLevel("").
		Build()

	if *options.SpecsUrl != "" || *options.LogEventUrl != "" ||
		*options.Environment != "" || *options.OutputLogLevel != "" {
		t.Error("expected all string fields to be empty")
	}
}

func TestAllValuesSet(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("https://example.com/specs").
		WithLogEventUrl("https://example.com/log").
		WithSpecsAdapterRef(12345).
		WithEventLoggingAdapterRef(12345).
		WithEnvironment("dev").
		WithEventLoggingFlushIntervalMs(3000).
		WithEventLoggingMaxQueueSize(10000).
		WithSpecsSyncIntervalMs(1500).
		WithOutputLogLevel("WARN").
		Build()

	if *options.Environment != "dev" {
		t.Errorf("expected Environment to be 'dev', got %v", options.Environment)
	}
	if options.EventLoggingFlushIntervalMs != 3000 {
		t.Errorf("expected EventLoggingFlushIntervalMs to be 3000, got %d", options.EventLoggingFlushIntervalMs)
	}
	if options.EventLoggingMaxQueueSize != 10000 {
		t.Errorf("expected EventLoggingMaxQueueSize to be 10000, got %d", options.EventLoggingMaxQueueSize)
	}
	if *options.OutputLogLevel != "WARN" {
		t.Errorf("expected OutputLogLevel to be 'WARN', got %v", options.OutputLogLevel)
	}
}
