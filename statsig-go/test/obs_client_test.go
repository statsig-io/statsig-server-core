package test

import (
	"encoding/json"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

type RecordedCall struct {
	MetricName string
	Value      float64
	Tags       map[string]string
}

type MockObservabilityClient struct {
	initCalled bool
	incCall    *RecordedCall
	gaugeCall  *RecordedCall
	distCall   *RecordedCall
	errorCall  *RecordedCall
}

func (m *MockObservabilityClient) Init() {
	m.initCalled = true
}

func (m *MockObservabilityClient) Increment(metricName string, value float64, tags map[string]string) {
	m.incCall = &RecordedCall{
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	}
}

func (m *MockObservabilityClient) Gauge(metricName string, value float64, tags map[string]string) {
	m.gaugeCall = &RecordedCall{
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	}
}

func (m *MockObservabilityClient) Dist(metricName string, value float64, tags map[string]string) {
	m.distCall = &RecordedCall{
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	}
}

func (m *MockObservabilityClient) Error(tag string, error string) {
	m.errorCall = &RecordedCall{
		MetricName: tag,
		Value:      0,
		Tags: map[string]string{
			"test_error": error,
		},
	}
}

func (m *MockObservabilityClient) ShouldEnableHighCardinalityForThisTag(tag string) bool {
	return true
}

func (m *MockObservabilityClient) GetFunctions() statsig_go.ObservabilityClientFunctions {
	return statsig_go.ObservabilityClientFunctions{
		Init:                                  m.Init,
		Increment:                             m.Increment,
		Gauge:                                 m.Gauge,
		Dist:                                  m.Dist,
		Error:                                 m.Error,
		ShouldEnableHighCardinalityForThisTag: m.ShouldEnableHighCardinalityForThisTag,
	}
}

func TestObservabilityClientInit(t *testing.T) {
	mock := &MockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	client.INTERNAL_testObservabilityClient("init", "", 0, "")

	if !mock.initCalled {
		t.Error("init_called should be true")
	}
}

func TestObservabilityClientIncrement(t *testing.T) {
	mock := &MockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	tags := mapToJson(t, map[string]string{
		"test_tag": "inc_test_value",
	})

	client.INTERNAL_testObservabilityClient("increment", "test_inc", 123, tags)

	if mock.incCall.MetricName != "test_inc" {
		t.Error("incCall.MetricName should be test_inc")
	}

	if mock.incCall.Value != 123 {
		t.Error("incCall.Value should be 123")
	}

	if mock.incCall.Tags["test_tag"] != "inc_test_value" {
		t.Error("incCall.Tags should be inc_test_value")
	}
}

func TestObservabilityClientGauge(t *testing.T) {
	mock := &MockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	tags := mapToJson(t, map[string]string{
		"test_tag": "gauge_test_value",
	})

	client.INTERNAL_testObservabilityClient("gauge", "test_gauge", 111, tags)

	if mock.gaugeCall.MetricName != "test_gauge" {
		t.Error("gaugeCall.MetricName should be test_gauge")
	}

	if mock.gaugeCall.Value != 111 {
		t.Error("gaugeCall.Value should be 111")
	}

	if mock.gaugeCall.Tags["test_tag"] != "gauge_test_value" {
		t.Error("gaugeCall.Tags should be gauge_test_value")
	}
}

func TestObservabilityClientDist(t *testing.T) {
	mock := &MockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	tags := mapToJson(t, map[string]string{
		"test_tag": "dist_test_value",
	})

	client.INTERNAL_testObservabilityClient("dist", "test_dist", 88, tags)

	if mock.distCall.MetricName != "test_dist" {
		t.Error("distCall.MetricName should be test_dist")
	}

	if mock.distCall.Value != 88 {
		t.Error("distCall.Value should be 88")
	}

	if mock.distCall.Tags["test_tag"] != "dist_test_value" {
		t.Error("distCall.Tags should be dist_test_value")
	}
}

func TestObservabilityClientError(t *testing.T) {
	mock := &MockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	tags := mapToJson(t, map[string]string{
		"test_error": "error_test_value",
	})

	client.INTERNAL_testObservabilityClient("error", "test_error", 0, tags)

	if mock.errorCall.MetricName != "test_error" {
		t.Error("errorCall.MetricName should be test_error")
	}

	if mock.errorCall.Tags["test_error"] != "error_test_value" {
		t.Error("errorCall.Tags should be error_test_value")
	}
}

func mapToJson(t *testing.T, tags map[string]string) string {
	json, err := json.Marshal(tags)
	if err != nil {
		t.Error("error marshalling tags", err)
	}

	return string(json)
}
