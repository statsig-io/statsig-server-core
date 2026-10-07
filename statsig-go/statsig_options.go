package statsig_go_core

import (
	"encoding/json"
	"fmt"
	"runtime"
)

const defaultSpecAdapterInitTimeoutMs uint64 = 3000

// Spec adapter types accepted by SpecAdapterConfig.AdapterType. Core treats an
// unrecognized value as SpecAdapterTypeNetworkHttp.
const (
	SpecAdapterTypeNetworkGrpcWebsocket = "network_grpc_websocket"
	SpecAdapterTypeNetworkHttp          = "network_http"
	SpecAdapterTypeDataStore            = "data_store"
)

// Authentication modes accepted by SpecAdapterConfig.AuthenticationMode. Only
// the gRPC websocket adapter uses these.
const (
	AuthenticationModeNone = "none"
	AuthenticationModeTls  = "tls"
	AuthenticationModeMtls = "mtls"
)

type StatsigOptions struct {
	ref uint64
	// obsClient carries the observability client from the builder to the
	// Statsig instance (see Statsig.obsClient), which is the durable owner.
	// Core holds only a Weak reference to the client, so an explicit strong
	// reference on the instance keeps it alive for the instance's lifetime.
	obsClient *ObservabilityClient
}

// SpecAdapterConfig configures the single spec source used by this SDK.
// Go accepts one adapter and does not fall back to HTTP. If that source is
// down at startup, initialization fails with "Failed to start any adapters".
type SpecAdapterConfig struct {
	// AdapterType is required. Use one of the SpecAdapterType* constants.
	// An empty or unrecognized value is treated as network_http by core.
	AdapterType string
	// SpecsUrl is required for SpecAdapterTypeNetworkGrpcWebsocket, for example
	// the Forward Proxy address. An empty value becomes "INVALID" in core.
	SpecsUrl string
	// InitTimeoutMs bounds the initial sync. 0 uses the SDK default of 3000 ms.
	InitTimeoutMs uint64
	// AuthenticationMode is one of the AuthenticationMode* constants.
	AuthenticationMode string
	// CaCertPath is used for tls and mtls.
	CaCertPath string
	// ClientCertPath and ClientKeyPath are used for mtls.
	ClientCertPath string
	ClientKeyPath  string
	// DomainName overrides the TLS server name for certificate verification.
	DomainName string
}

type StatsigOptionsBuilder struct {
	SpecsUrl                    *string `json:"specs_url,omitempty"`
	LogEventUrl                 *string `json:"log_event_url,omitempty"`
	Environment                 *string `json:"environment,omitempty"`
	ServiceName                 *string `json:"service_name,omitempty"`
	EventLoggingFlushIntervalMs *int32  `json:"event_logging_flush_interval_ms,omitempty"`
	EventLoggingMaxQueueSize    *int32  `json:"event_logging_max_queue_size,omitempty"`
	ExposureDedupeMaxKeys       *int32  `json:"exposure_dedupe_max_keys,omitempty"`
	SpecsSyncIntervalMs         *int32  `json:"specs_sync_interval_ms,omitempty"`
	OutputLogLevel              *string `json:"output_log_level,omitempty"`
	DisableCountryLookup        *bool   `json:"disable_country_lookup,omitempty"`
	DisableUserAgentParsing     *bool   `json:"disable_user_agent_parsing,omitempty"`
	WaitForCountryLookupInit    *bool   `json:"wait_for_country_lookup_init,omitempty"`
	WaitForUserAgentInit        *bool   `json:"wait_for_user_agent_init,omitempty"`
	EnableIdLists               *bool   `json:"enable_id_lists,omitempty"`
	EnableDcsDeltas             *bool   `json:"enable_dcs_deltas,omitempty"`
	IdListsUrl                  *string `json:"id_lists_url,omitempty"`
	IdListsSyncIntervalMs       *int32  `json:"id_lists_sync_interval_ms,omitempty"`
	IdListsRequestTimeoutMs     *int64  `json:"id_lists_request_timeout_ms,omitempty"`
	DisableAllLogging           *bool   `json:"disable_all_logging,omitempty"`
	DisableNetwork              *bool   `json:"disable_network,omitempty"`
	GlobalCustomFields          *string `json:"global_custom_fields,omitempty"`
	ObservabilityClientRef      *uint64 `json:"observability_client_ref,omitempty"`
	DataStoreRef                *uint64 `json:"data_store_ref,omitempty"`
	PersistentStorageRef        *uint64 `json:"persistent_storage_ref,omitempty"`
	InitTimeoutMs               *int32  `json:"init_timeout_ms,omitempty"`
	FallbackToStatsigApi        *bool   `json:"fallback_to_statsig_api,omitempty"`

	SpecAdapterType               *string `json:"spec_adapter_type,omitempty"`
	SpecAdapterUrl                *string `json:"spec_adapter_url,omitempty"`
	SpecAdapterInitTimeoutMs      *uint64 `json:"spec_adapter_init_timeout_ms,omitempty"`
	SpecAdapterAuthenticationMode *string `json:"spec_adapter_authentication_mode,omitempty"`
	SpecAdapterCaCertPath         *string `json:"spec_adapter_ca_cert_path,omitempty"`
	SpecAdapterClientCertPath     *string `json:"spec_adapter_client_cert_path,omitempty"`
	SpecAdapterClientKeyPath      *string `json:"spec_adapter_client_key_path,omitempty"`
	SpecAdapterDomainName         *string `json:"spec_adapter_domain_name,omitempty"`

	// observabilityClient is retained (unexported, so it is never marshaled)
	// solely to keep the client alive; see StatsigOptions.obsClient.
	observabilityClient *ObservabilityClient
}

func NewOptionsBuilder() *StatsigOptionsBuilder {
	return &StatsigOptionsBuilder{}
}

func (o *StatsigOptionsBuilder) WithSpecsUrl(specsUrl string) *StatsigOptionsBuilder {
	o.SpecsUrl = &specsUrl
	return o
}

// WithSpecAdapterConfig sets the spec adapter and its optional connection settings.
func (o *StatsigOptionsBuilder) WithSpecAdapterConfig(config SpecAdapterConfig) *StatsigOptionsBuilder {
	initTimeoutMs := config.InitTimeoutMs
	if initTimeoutMs == 0 {
		initTimeoutMs = defaultSpecAdapterInitTimeoutMs
	}

	o.SpecAdapterType = &config.AdapterType
	o.SpecAdapterInitTimeoutMs = &initTimeoutMs
	o.SpecAdapterUrl = optionalString(config.SpecsUrl)
	o.SpecAdapterAuthenticationMode = optionalString(config.AuthenticationMode)
	o.SpecAdapterCaCertPath = optionalString(config.CaCertPath)
	o.SpecAdapterClientCertPath = optionalString(config.ClientCertPath)
	o.SpecAdapterClientKeyPath = optionalString(config.ClientKeyPath)
	o.SpecAdapterDomainName = optionalString(config.DomainName)
	return o
}

func optionalString(value string) *string {
	if value == "" {
		return nil
	}
	return &value
}

func (o *StatsigOptionsBuilder) WithLogEventUrl(logEventUrl string) *StatsigOptionsBuilder {
	o.LogEventUrl = &logEventUrl
	return o
}

func (o *StatsigOptionsBuilder) WithEnvironment(environment string) *StatsigOptionsBuilder {
	o.Environment = &environment
	return o
}

func (o *StatsigOptionsBuilder) WithServiceName(serviceName string) *StatsigOptionsBuilder {
	o.ServiceName = &serviceName
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

func (o *StatsigOptionsBuilder) WithExposureDedupeMaxKeys(exposureDedupeMaxKeys int32) *StatsigOptionsBuilder {
	o.ExposureDedupeMaxKeys = &exposureDedupeMaxKeys
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

func (o *StatsigOptionsBuilder) WithEnableDcsDeltas(enableDcsDeltas bool) *StatsigOptionsBuilder {
	o.EnableDcsDeltas = &enableDcsDeltas
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

func (o *StatsigOptionsBuilder) WithIdListsRequestTimeoutMs(idListsRequestTimeoutMs int64) *StatsigOptionsBuilder {
	// Negative values can't deserialize into the core's Option<u64>; leave unset
	// so the network provider's default timeout applies.
	if idListsRequestTimeoutMs >= 0 {
		o.IdListsRequestTimeoutMs = &idListsRequestTimeoutMs
	}
	return o
}

func (o *StatsigOptionsBuilder) WithObservabilityClient(observabilityClient *ObservabilityClient) *StatsigOptionsBuilder {
	if observabilityClient == nil {
		o.ObservabilityClientRef = nil
		o.observabilityClient = nil
		return o
	}
	o.ObservabilityClientRef = &observabilityClient.ref
	// Retain the client so it (and thus the Rust registry's strong Arc) stays
	// alive; core only holds a Weak reference to it.
	o.observabilityClient = observabilityClient
	return o
}

func (o *StatsigOptionsBuilder) WithDataStore(dataStore *DataStore) *StatsigOptionsBuilder {
	o.DataStoreRef = &dataStore.ref
	return o
}

func (o *StatsigOptionsBuilder) WithPersistentStorage(persistentStorage *PersistentStorage) *StatsigOptionsBuilder {
	o.PersistentStorageRef = &persistentStorage.ref
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
		ref:       ref,
		obsClient: o.observabilityClient,
	}

	runtime.SetFinalizer(options, func(obj *StatsigOptions) {
		GetFFI().statsig_options_release(obj.ref)
	})

	return options, nil
}
