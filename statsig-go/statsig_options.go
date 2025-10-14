package statsig_go_core

import (
	"encoding/json"
	"fmt"
)

type StatsigOptions struct {
	ref uint64
}

type StatsigOptionsBuilder struct {
	SpecsUrl                    *string `json:"specs_url,omitempty"`
	LogEventUrl                 *string `json:"log_event_url,omitempty"`
	Environment                 *string `json:"environment,omitempty"`
	EventLoggingFlushIntervalMs *int32  `json:"event_logging_flush_interval_ms,omitempty"`
	EventLoggingMaxQueueSize    *int32  `json:"event_logging_max_queue_size,omitempty"`
	SpecsSyncIntervalMs         *int32  `json:"specs_sync_interval_ms,omitempty"`
	OutputLogLevel              *string `json:"output_log_level,omitempty"`
	DisableCountryLookup        *bool   `json:"disable_country_lookup,omitempty"`
	DisableUserAgentParsing     *bool   `json:"disable_user_agent_parsing,omitempty"`
	WaitForCountryLookupInit    *bool   `json:"wait_for_country_lookup_init,omitempty"`
	WaitForUserAgentInit        *bool   `json:"wait_for_user_agent_init,omitempty"`
	EnableIdLists               *bool   `json:"enable_id_lists,omitempty"`
	IdListsUrl                  *string `json:"id_lists_url,omitempty"`
	IdListsSyncIntervalMs       *int32  `json:"id_lists_sync_interval_ms,omitempty"`
	DisableAllLogging           *bool   `json:"disable_all_logging,omitempty"`
	DisableNetwork              *bool   `json:"disable_network,omitempty"`
	GlobalCustomFields          *string `json:"global_custom_fields,omitempty"`
	ObservabilityClientRef      *uint64 `json:"observability_client_ref,omitempty"`
	DataStoreRef                *uint64 `json:"data_store_ref,omitempty"`
	InitTimeoutMs               *int32  `json:"init_timeout_ms,omitempty"`
	FallbackToStatsigApi        *bool   `json:"fallback_to_statsig_api,omitempty"`
}

func NewOptionsBuilder() *StatsigOptionsBuilder {
	return &StatsigOptionsBuilder{}
}

func (o *StatsigOptionsBuilder) WithSpecsUrl(specsUrl string) *StatsigOptionsBuilder {
	o.SpecsUrl = &specsUrl
	return o
}

func (o *StatsigOptionsBuilder) WithLogEventUrl(logEventUrl string) *StatsigOptionsBuilder {
	o.LogEventUrl = &logEventUrl
	return o
}

func (o *StatsigOptionsBuilder) WithEnvironment(environment string) *StatsigOptionsBuilder {
	o.Environment = &environment
	return o
}

func (o *StatsigOptionsBuilder) WithEventLoggingFlushIntervalMs(eventLoggingFlushIntervalMs int32) *StatsigOptionsBuilder {
	o.EventLoggingFlushIntervalMs = &eventLoggingFlushIntervalMs
	return o
}

func (o *StatsigOptionsBuilder) WithEventLoggingMaxQueueSize(eventLoggingMaxQueueSize int32) *StatsigOptionsBuilder {
	o.EventLoggingMaxQueueSize = &eventLoggingMaxQueueSize
	return o
}

func (o *StatsigOptionsBuilder) WithSpecsSyncIntervalMs(specsSyncIntervalMs int32) *StatsigOptionsBuilder {
	o.SpecsSyncIntervalMs = &specsSyncIntervalMs
	return o
}

func (o *StatsigOptionsBuilder) WithOutputLogLevel(outputLogLevel string) *StatsigOptionsBuilder {
	o.OutputLogLevel = &outputLogLevel
	return o
}

func (o *StatsigOptionsBuilder) WithDisableCountryLookup(disableCountryLookup bool) *StatsigOptionsBuilder {
	o.DisableCountryLookup = &disableCountryLookup
	return o
}

func (o *StatsigOptionsBuilder) WithDisableUserAgentParsing(disableUserAgentParsing bool) *StatsigOptionsBuilder {
	o.DisableUserAgentParsing = &disableUserAgentParsing
	return o
}

func (o *StatsigOptionsBuilder) WithWaitForCountryLookupInit(waitForCountryLookupInit bool) *StatsigOptionsBuilder {
	o.WaitForCountryLookupInit = &waitForCountryLookupInit
	return o
}

func (o *StatsigOptionsBuilder) WithWaitForUserAgentInit(waitForUserAgentInit bool) *StatsigOptionsBuilder {
	o.WaitForUserAgentInit = &waitForUserAgentInit
	return o
}

func (o *StatsigOptionsBuilder) WithDisableAllLogging(disableAllLogging bool) *StatsigOptionsBuilder {
	o.DisableAllLogging = &disableAllLogging
	return o
}

func (o *StatsigOptionsBuilder) WithDisableNetwork(disableNetwork bool) *StatsigOptionsBuilder {
	o.DisableNetwork = &disableNetwork
	return o
}

func (o *StatsigOptionsBuilder) WithEnableIdLists(enableIdLists bool) *StatsigOptionsBuilder {
	o.EnableIdLists = &enableIdLists
	return o
}

func (o *StatsigOptionsBuilder) WithIdListsUrl(idListsUrl string) *StatsigOptionsBuilder {
	o.IdListsUrl = &idListsUrl
	return o
}

func (o *StatsigOptionsBuilder) WithIdListsSyncIntervalMs(idListsSyncIntervalMs int32) *StatsigOptionsBuilder {
	o.IdListsSyncIntervalMs = &idListsSyncIntervalMs
	return o
}

func (o *StatsigOptionsBuilder) WithDataStore(dataStore *DataStore) *StatsigOptionsBuilder {
	o.DataStoreRef = &dataStore.ref
	return o
}

func (o *StatsigOptionsBuilder) Build() (*StatsigOptions, error) {
	data, err := json.Marshal(o)
	if err != nil {
		return nil, err
	}

	ref := GetFFI().statsig_options_create_from_data(
		string(data),
	)

	if ref == 0 {
		return nil, fmt.Errorf("failed to create StatsigOptions")
	}

	options := &StatsigOptions{
		ref,
	}

	return options, nil
}
