package tests

import (
	"encoding/json"
	"testing"
	"time"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

type MockObservabilityClient struct {
	init_called  bool
	dist_called  bool
	error_called bool
	metrics      []MetricRecord
}

type MetricRecord struct {
	Type       string
	MetricName string
	Value      interface{}
	Tags       map[string]string
}

func NewMockObservabilityClient() *MockObservabilityClient {
	return &MockObservabilityClient{
		metrics: make([]MetricRecord, 0),
	}
}

func (obs *MockObservabilityClient) Init() error {
	obs.init_called = true
	return nil
}

func (obs *MockObservabilityClient) Increment(metricName string, value float64, tags map[string]string) error {
	obs.metrics = append(obs.metrics, MetricRecord{
		Type:       "increment",
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	})
	return nil
}

func (obs *MockObservabilityClient) Gauge(metricName string, value float64, tags map[string]string) error {
	obs.metrics = append(obs.metrics, MetricRecord{
		Type:       "gauge",
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	})
	return nil
}

func (obs *MockObservabilityClient) Distribution(metricName string, value float64, tags map[string]string) error {
	obs.dist_called = true
	obs.metrics = append(obs.metrics, MetricRecord{
		Type:       "distribution",
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	})
	return nil
}

func (obs *MockObservabilityClient) ShouldEnableHighCardinalityForThisTag(tag string) bool {
	return true
}

func (obs *MockObservabilityClient) Error(tag string, error string) {
	obs.error_called = true
	obs.metrics = append(obs.metrics, MetricRecord{
		Type:       "error",
		MetricName: tag,
		Value:      error,
		Tags:       nil,
	})
}

func TestObservabilityClientUsage(t *testing.T) {
	scrapi := serverSetup("eval_proj_dcs.json")

	observabilityClient := NewMockObservabilityClient()

	options := statsig.NewStatsigOptionsBuilder().
		WithObservabilityClient(observabilityClient).
		WithSpecsUrl(scrapi.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapi.GetUrlForEndpoint("/v1/log_event")).
		WithSpecsSyncIntervalMs(1).
		WithOutputLogLevel("ERROR").
		Build()

	statsigInstance, err := statsig.NewStatsig("secret-key", *options)
	if err != nil {
		t.Fatalf("Failed to create Statsig instance: %v", err)
	}

	user := statsig.NewStatsigUserBuilder().WithUserID("test-user").Build()
	_, err = statsigInstance.Initialize()

	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}
	defer statsigInstance.Shutdown()

	statsigInstance.GetFeatureGate(*user, "test-gate", nil)
	statsigInstance.FlushEvents()

	time.Sleep(200 * time.Millisecond)

	if !observabilityClient.init_called {
		t.Error("Expected init() to have been called")
	}

	var initDistEvent *MetricRecord
	for _, metric := range observabilityClient.metrics {
		if metric.Type == "distribution" && metric.MetricName == "statsig.sdk.initialization" {
			initDistEvent = &metric
			break
		}
	}

	if initDistEvent == nil {
		t.Error("Expected distribution() to have been called for statsig.sdk.initialization")
	} else {
		if _, ok := initDistEvent.Value.(float64); !ok {
			t.Errorf("Expected distribution value to be float64, got %T", initDistEvent.Value)
		}

		expectedTags := map[string]string{
			"success":         "true",
			"store_populated": "true",
			"source":          "Network",
		}

		for key, expectedValue := range expectedTags {
			if actualValue, exists := initDistEvent.Tags[key]; !exists || actualValue != expectedValue {
				t.Errorf("Expected tag %s to be %s, got %s", key, expectedValue, actualValue)
			}
		}
	}
}

func TestErrorCallbackUsage(t *testing.T) {
	observabilityClient := NewMockObservabilityClient()

	options := statsig.NewStatsigOptionsBuilder().
		WithObservabilityClient(observabilityClient).
		Build()

	statsigInstance, err := statsig.NewStatsig("secret-key", *options)
	if err != nil {
		t.Fatalf("Failed to create Statsig instance: %v", err)
	}

	_, err = statsigInstance.Initialize()
	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}
	defer statsigInstance.Shutdown()

	time.Sleep(200 * time.Millisecond)

	if !observabilityClient.error_called {
		t.Error("Expected error() to have been called")
	}

	if len(observabilityClient.metrics) < 1 {
		t.Errorf("Expected at least 1 metric, got %d", len(observabilityClient.metrics))
	}

	var errorEvent *MetricRecord
	for _, metric := range observabilityClient.metrics {
		if metric.Type == "error" {
			errorEvent = &metric
			break
		}
	}

	if errorEvent == nil {
		t.Error("Expected error() to have been called")
	} else {
		if _, ok := errorEvent.Value.(string); !ok {
			t.Errorf("Expected error value to be string, got %T", errorEvent.Value)
		}
	}
}

func TestMetricWithHighCardinality(t *testing.T) {
	scrapi := serverSetup("eval_proj_dcs.json")

	observabilityClient := NewMockObservabilityClient()

	options := statsig.NewStatsigOptionsBuilder().
		WithObservabilityClient(observabilityClient).
		WithSpecsUrl(scrapi.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapi.GetUrlForEndpoint("/v1/log_event")).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("ERROR").
		Build()

	statsigInstance, err := statsig.NewStatsig("secret-key", *options)

	if err != nil {
		t.Fatalf("Failed to create Statsig instance: %v", err)
	}
	_, _ = statsigInstance.Initialize()

	user := statsig.NewStatsigUserBuilder().WithUserID("test-user").Build()

	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}
	defer statsigInstance.Shutdown()

	statsigInstance.GetFeatureGate(*user, "test-gate", nil)
	statsigInstance.FlushEvents()

	time.Sleep(3 * time.Second)

	if !observabilityClient.init_called {
		t.Error("Expected init() to have been called")
	}

	var configPropEvent *MetricRecord
	for _, metric := range observabilityClient.metrics {
		if metric.Type == "distribution" && metric.MetricName == "statsig.sdk.config_propagation_diff" {
			configPropEvent = &metric
			break
		}
	}

	if configPropEvent == nil {
		t.Error("Expected distribution() to have been called for statsig.sdk.config_propagation_diff")
	} else {
		if _, ok := configPropEvent.Value.(float64); !ok {
			t.Errorf("Expected distribution value to be float64, got %T", configPropEvent.Value)
		}

		if lcutStr, exists := configPropEvent.Tags["lcut"]; exists {
			if _, err := json.Marshal(lcutStr); err != nil {
				t.Errorf("Expected lcut tag to be valid JSON string, got %s", lcutStr)
			}
		} else {
			t.Error("Expected lcut tag to exist")
		}

		if prevLcutStr, exists := configPropEvent.Tags["prev_lcut"]; exists {
			if _, err := json.Marshal(prevLcutStr); err != nil {
				t.Errorf("Expected prev_lcut tag to be valid JSON string, got %s", prevLcutStr)
			}
		} else {
			t.Error("Expected prev_lcut tag to exist")
		}
	}
}
