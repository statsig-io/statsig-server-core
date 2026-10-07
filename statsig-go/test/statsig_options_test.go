package test

import (
	"encoding/json"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestStatsigOptionsBuilder(t *testing.T) {
	_, err := statsig_go.NewOptionsBuilder().WithSpecsUrl("http://localhost:3000/specs").Build()

	if err != nil {
		t.Errorf("error creating StatsigOptions: %v", err)
	}
}

func TestStatsigOptionsBuilderArgs(t *testing.T) {
	builder := statsig_go.StatsigOptionsBuilder{
		SpecsUrl:                    ptr("http://localhost:3000/specs"),
		LogEventUrl:                 ptr("http://localhost:3000/events"),
		Environment:                 ptr("production"),
		ServiceName:                 ptr("statsig-go-service"),
		SpecsSyncIntervalMs:         ptr(int32(1000)),
		EventLoggingFlushIntervalMs: ptr(int32(2000)),
		EventLoggingMaxQueueSize:    ptr(int32(5000)),
		WaitForCountryLookupInit:    ptr(true),
		WaitForUserAgentInit:        ptr(true),
	}

	_, err := builder.Build()

	if err != nil {
		t.Errorf("error creating StatsigOptions: %v", err)
	}
}

func TestWithSpecAdapterConfigSerializesFFIFields(t *testing.T) {
	adapterTypes := []string{
		statsig_go.SpecAdapterTypeNetworkHttp,
		statsig_go.SpecAdapterTypeNetworkGrpcWebsocket,
	}
	for _, adapterType := range adapterTypes {
		t.Run(adapterType, func(t *testing.T) {
			builder := statsig_go.NewOptionsBuilder().WithSpecAdapterConfig(statsig_go.SpecAdapterConfig{
				AdapterType:        adapterType,
				SpecsUrl:           "https://forward-proxy.example.com",
				InitTimeoutMs:      5000,
				AuthenticationMode: statsig_go.AuthenticationModeTls,
				CaCertPath:         "/certs/ca.pem",
				ClientCertPath:     "/certs/client.pem",
				ClientKeyPath:      "/certs/client-key.pem",
				DomainName:         "forward-proxy.example.com",
			})

			data, err := json.Marshal(builder)
			if err != nil {
				t.Fatalf("failed to marshal options: %v", err)
			}

			var got map[string]any
			if err := json.Unmarshal(data, &got); err != nil {
				t.Fatalf("failed to unmarshal options: %v", err)
			}

			want := map[string]any{
				"spec_adapter_type":                adapterType,
				"spec_adapter_url":                 "https://forward-proxy.example.com",
				"spec_adapter_init_timeout_ms":     float64(5000),
				"spec_adapter_authentication_mode": "tls",
				"spec_adapter_ca_cert_path":        "/certs/ca.pem",
				"spec_adapter_client_cert_path":    "/certs/client.pem",
				"spec_adapter_client_key_path":     "/certs/client-key.pem",
				"spec_adapter_domain_name":         "forward-proxy.example.com",
			}
			for key, wantValue := range want {
				if got[key] != wantValue {
					t.Errorf("%s = %v, want %v", key, got[key], wantValue)
				}
			}
			if _, ok := got["spec_adapter_specs_url"]; ok {
				t.Error("serialized unsupported spec_adapter_specs_url; the FFI expects spec_adapter_url")
			}
		})
	}
}

func TestWithSpecAdapterConfigDefaultsInitTimeout(t *testing.T) {
	builder := statsig_go.NewOptionsBuilder().WithSpecAdapterConfig(statsig_go.SpecAdapterConfig{
		AdapterType: statsig_go.SpecAdapterTypeNetworkGrpcWebsocket,
	})

	if got := *builder.SpecAdapterInitTimeoutMs; got != 3000 {
		t.Errorf("spec adapter init timeout = %d, want 3000", got)
	}
	if builder.SpecAdapterUrl != nil {
		t.Errorf("spec adapter URL = %v, want nil", *builder.SpecAdapterUrl)
	}
	for name, field := range map[string]*string{
		"authentication mode": builder.SpecAdapterAuthenticationMode,
		"CA cert path":        builder.SpecAdapterCaCertPath,
		"client cert path":    builder.SpecAdapterClientCertPath,
		"client key path":     builder.SpecAdapterClientKeyPath,
		"domain name":         builder.SpecAdapterDomainName,
	} {
		if field != nil {
			t.Errorf("spec adapter %s = %q, want nil", name, *field)
		}
	}
}

func ptr[T any](v T) *T {
	return &v
}
