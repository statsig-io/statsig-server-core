package test

import (
	"encoding/json"
	"os"
	"runtime"
	"sync"
	"testing"
	"time"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

type recordedCall struct {
	MetricName string
	Value      float64
	Tags       map[string]string
}

type mockObservabilityClient struct {
	initCalled bool
	incCall    *recordedCall
	gaugeCall  *recordedCall
	distCall   *recordedCall
	errorCall  *recordedCall
}

func (m *mockObservabilityClient) Init() {
	m.initCalled = true
}

func (m *mockObservabilityClient) Increment(metricName string, value float64, tags map[string]string) {
	m.incCall = &recordedCall{
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	}
}

func (m *mockObservabilityClient) Gauge(metricName string, value float64, tags map[string]string) {
	m.gaugeCall = &recordedCall{
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	}
}

func (m *mockObservabilityClient) Dist(metricName string, value float64, tags map[string]string) {
	m.distCall = &recordedCall{
		MetricName: metricName,
		Value:      value,
		Tags:       tags,
	}
}

func (m *mockObservabilityClient) Error(tag string, error string) {
	m.errorCall = &recordedCall{
		MetricName: tag,
		Value:      0,
		Tags: map[string]string{
			"test_error": error,
		},
	}
}

func (m *mockObservabilityClient) ShouldEnableHighCardinalityForThisTag(tag string) bool {
	return true
}

func (m *mockObservabilityClient) GetFunctions() statsig_go.ObservabilityClientFunctions {
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
	mock := &mockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	client.INTERNAL_testObservabilityClient("init", "", 0, "")

	if !mock.initCalled {
		t.Error("init_called should be true")
	}
}

func TestObservabilityClientIncrement(t *testing.T) {
	mock := &mockObservabilityClient{}

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
	mock := &mockObservabilityClient{}

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
	mock := &mockObservabilityClient{}

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
	mock := &mockObservabilityClient{}

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

func TestObservabilityClientWiredIntoOptions(t *testing.T) {
	mock := &mockObservabilityClient{}

	client := statsig_go.NewObservabilityClient(mock.GetFunctions())

	builder := statsig_go.NewOptionsBuilder().
		WithObservabilityClient(client)

	if builder.ObservabilityClientRef == nil {
		t.Fatal("expected ObservabilityClientRef to be set, got nil")
	}
	if *builder.ObservabilityClientRef == 0 {
		t.Fatal("expected ObservabilityClientRef to be non-zero")
	}

	options, err := builder.Build()

	if err != nil {
		t.Fatalf("failed to build options with observability client: %v", err)
	}

	if options == nil {
		t.Fatal("expected non-nil options")
	}
}

func TestObservabilityClientNilIsSafe(t *testing.T) {
	builder := statsig_go.NewOptionsBuilder().
		WithObservabilityClient(nil)

	if builder.ObservabilityClientRef != nil {
		t.Fatal("expected ObservabilityClientRef to remain nil when client is nil")
	}
}

// metricRecorder is a thread-safe observability client sink. The SDK invokes
// these callbacks from a Rust worker thread, so access must be synchronized.
type metricRecorder struct {
	mu      sync.Mutex
	metrics map[string]int
}

func newMetricRecorder() *metricRecorder {
	return &metricRecorder{metrics: map[string]int{}}
}

func (r *metricRecorder) record(metric string) {
	r.mu.Lock()
	r.metrics[metric]++
	r.mu.Unlock()
}

func (r *metricRecorder) count(metric string) int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.metrics[metric]
}

func (r *metricRecorder) functions() statsig_go.ObservabilityClientFunctions {
	return statsig_go.ObservabilityClientFunctions{
		Init:                                  func() {},
		Increment:                             func(m string, _ float64, _ map[string]string) { r.record(m) },
		Gauge:                                 func(m string, _ float64, _ map[string]string) { r.record(m) },
		Dist:                                  func(m string, _ float64, _ map[string]string) { r.record(m) },
		Error:                                 func(_ string, _ string) {},
		ShouldEnableHighCardinalityForThisTag: func(_ string) bool { return true },
	}
}

// TestObservabilityClientDeliveryEndToEnd verifies observability metrics flow
// all the way through a live Statsig instance to the client:
// builder -> options -> core ops-stats listener -> FFI callback -> Go. It drops
// the client and options references and forces GC before initializing so it
// also exercises the instance-lifetime retention added for observability
// (Statsig.obsClient), which mirrors the strong-owner pattern in the Node and
// Python bindings and guards against the Weak reference core holds being
// upgraded on a collected client.
//
// Caveat: this asserts delivery, it does not isolate the retention fix. purego
// pins the client's FFI callback closures in its global callback registry
// (cbs.funcs, never freed) for the process lifetime, so the client is not
// actually collectable here regardless of the Go-side retention.
func TestObservabilityClientDeliveryEndToEnd(t *testing.T) {
	dcs, err := os.ReadFile("../../statsig-rust/tests/data/eval_proj_dcs.json")
	if err != nil {
		t.Fatalf("error reading dcs data: %v", err)
	}

	scrapi := NewMockScrapi()
	defer scrapi.Close()
	scrapi.Stub("GET", "/v2/download_config_specs/secret-123.json", StubResponse{
		Status: 200,
		Body:   dcs,
	})
	scrapi.Stub("POST", "/v1/log_event", StubResponse{
		Status: 200,
		Body:   []byte(`{"success": true}`),
	})

	rec := newMetricRecorder()

	// Build and create inside a closure so the *ObservabilityClient and
	// *StatsigOptions references go out of scope, leaving the Statsig instance
	// as the only retainer of the client.
	statsig := func() *statsig_go.Statsig {
		client := statsig_go.NewObservabilityClient(rec.functions())

		opts, err := statsig_go.NewOptionsBuilder().
			WithObservabilityClient(client).
			WithSpecsUrl(scrapi.URL() + "/v2/download_config_specs").
			WithLogEventUrl(scrapi.URL() + "/v1/log_event").
			Build()
		if err != nil {
			t.Fatalf("error creating StatsigOptions: %v", err)
		}

		s, err := statsig_go.NewStatsigWithOptions("secret-123", opts)
		if err != nil {
			t.Fatalf("error creating Statsig: %v", err)
		}
		return s
	}()

	// Force GC (and give finalizers time to run) with only the Statsig instance
	// left holding the client, exercising the retention path.
	for i := 0; i < 3; i++ {
		runtime.GC()
		time.Sleep(20 * time.Millisecond)
	}

	statsig.Initialize()
	defer statsig.Shutdown()

	// The init dist metric is emitted asynchronously by the ops-stats listener,
	// which upgrades its Weak reference to the client on each event.
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		if rec.count("statsig.sdk.initialization") > 0 {
			return
		}
		time.Sleep(50 * time.Millisecond)
	}

	t.Fatal("observability client never received the statsig.sdk.initialization dist metric " +
		"through a live Statsig instance")
}

func mapToJson(t *testing.T, tags map[string]string) string {
	json, err := json.Marshal(tags)
	if err != nil {
		t.Error("error marshalling tags", err)
	}

	return string(json)
}
