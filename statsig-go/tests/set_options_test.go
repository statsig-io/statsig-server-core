package tests

import (
	"testing"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

func TestStatsigOptionsBasic(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("https://example.com/specs").
		WithLogEventUrl("https://example.com/log").
		WithEnvironment("production").
		WithEventLoggingFlushIntervalMs(2000).
		WithEventLoggingMaxQueueSize(5000).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		WithDisableCountryLookup(true).
		WithDisableUserAgentParsing(true).
		WithWaitForCountryLookupInit(false).
		WithEnableIdLists(false).
		WithIdListsUrl("").
		WithIDListsSyncIntervalMs(-1).
		WithDisableAllLogging(false).
		WithGlobalCustomFields("").
		Build()

	if *options.SpecsUrl != "https://example.com/specs" {
		t.Errorf("expected SpecsUrl to be 'https://example.com/specs', got %v", options.SpecsUrl)
	}
	if *options.LogEventUrl != "https://example.com/log" {
		t.Errorf("expected LogEventUrl to be 'https://example.com/log', got %v", options.LogEventUrl)
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
	if *options.EnableIdLists != false {
		t.Errorf("expected EnableIdLists to be 'false', got %v", *options.EnableIdLists)
	}
	if *options.IdListsUrl != "" {
		t.Errorf("expected IdListsUrl to be '', got %v", *options.IdListsUrl)
	}
	if options.IdListsSyncIntervalMs != -1 {
		t.Errorf("expected IDListsSyncIntervalMs to be -1, got %d", options.IdListsSyncIntervalMs)
	}
	if *options.DisableAllLogging != false {
		t.Errorf("expected DisableAllLogging to be 'false', got %v", *options.DisableAllLogging)
	}
	if *options.GlobalCustomFields != "" {
		t.Errorf("expected GlobalCustomFields to be '', got %v", *options.GlobalCustomFields)
	}

}

func TestBuilderSetNumericValues(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsSyncIntervalMs(12345).
		WithEventLoggingFlushIntervalMs(67890).
		WithEventLoggingMaxQueueSize(99999).
		WithIDListsSyncIntervalMs(30000).
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
	if options.IdListsSyncIntervalMs != 30000 {
		t.Errorf("expected IdListsSyncIntervalMs to be 30000, got %d", options.IdListsSyncIntervalMs)
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
		WithIdListsUrl("").
		Build()

	if *options.SpecsUrl != "" || *options.LogEventUrl != "" ||
		*options.Environment != "" || *options.OutputLogLevel != "" || *options.IdListsUrl != "" {
		t.Error("expected all string fields to be empty")
	}
}

func TestAllValuesSet(t *testing.T) {
	options := statsig.NewStatsigOptionsBuilder().
		WithSpecsUrl("https://example.com/specs").
		WithLogEventUrl("https://example.com/log").
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
