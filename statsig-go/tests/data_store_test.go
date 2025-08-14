package tests

import (
	"encoding/json"
	"fmt"
	"reflect"
	"sync"
	"testing"
	"time"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

// MockDataStore models the Python MockDataStore class
type MockDataStore struct {
	store            map[string]string
	mu               sync.RWMutex
	init_called      bool
	content_set      string
	get_called_count int
	should_poll      bool
	test_param       string
	dcs_content      string
}

func NewMockDataStore(testParam string) *MockDataStore {
	dcs_content := string(LoadTestData("eval_proj_dcs.json"))

	return &MockDataStore{
		store:       make(map[string]string),
		test_param:  testParam,
		dcs_content: dcs_content,
	}
}

func (d *MockDataStore) Get(key string) string {
	d.mu.Lock()
	fmt.Println("Getting MockDataStore")
	d.get_called_count++
	d.mu.Unlock()

	res := map[string]interface{}{
		"result": d.dcs_content,
		"time":   1234567890,
	}

	json, _ := json.Marshal(res)
	d.mu.RLock()
	defer d.mu.RUnlock()
	return string(json)
}

func (d *MockDataStore) Set(key string, value string) {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.content_set = value
	fmt.Println("Setting MockDataStore")
}

func (d *MockDataStore) Initialize() {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.init_called = true
	fmt.Println("Initializing MockDataStore")
}

func (d *MockDataStore) Shutdown() {
	fmt.Println("Shutting down MockDataStore")
}

func (d *MockDataStore) ShouldBeUsedForQueryingUpdates(key string) bool {
	fmt.Printf("Checking if polling updates are supported for key: %s: should_poll=%t\n", key, d.should_poll)
	return d.should_poll
}

// Test data store usage with test parameter
func TestDataStoreUsageGetWithTestParam(t *testing.T) {
	dataStore := NewMockDataStore("test_param")
	if dataStore.test_param != "test_param" {
		t.Errorf("Expected test_param to be 'test_param', got '%s'", dataStore.test_param)
	}
}

func updateDCSContent(originalData []byte, t *testing.T) []byte {
	var updatedData map[string]interface{}
	if err := json.Unmarshal(originalData, &updatedData); err != nil {
		t.Fatalf("Failed to parse original data: %v", err)
	}

	if timeVal, exists := updatedData["time"]; exists {
		if timeFloat, ok := timeVal.(float64); ok {
			updatedData["time"] = timeFloat + 10
		}
	}

	updatedDataBytes, _ := json.Marshal(updatedData)
	return updatedDataBytes
}

// Test data store usage with get operations
func TestDataStoreUsageGet(t *testing.T) {
	originalData := LoadTestData("eval_proj_dcs.json")
	updatedDataBytes := updateDCSContent(originalData, t)

	scrapi := serverSetup("eval_proj_dcs.json", &updatedDataBytes)

	dataStore := NewMockDataStore("test_param")
	dataStore.should_poll = true

	options := statsig.NewStatsigOptionsBuilder().
		WithDataStore(dataStore).
		WithSpecsUrl(scrapi.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapi.GetUrlForEndpoint("/v1/log_event")).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("INFO").
		Build()

	// Create Statsig instance
	statsigInstance, _ := statsig.NewStatsig("secret-key", *options)
	user := statsig.NewStatsigUserBuilder().WithUserID("test_user_id").Build()

	// Initialize
	_, err := statsigInstance.Initialize()
	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}
	defer statsigInstance.Shutdown()

	gate := statsigInstance.GetFeatureGate(*user, "test_public", nil)

	statsigInstance.FlushEvents()

	if !dataStore.init_called {
		t.Error("Expected data store initialize to be called")
	}

	time.Sleep(3 * time.Second)

	if gate.EvaluationDetails.Reason != "Adapter(DataStore):Recognized" {
		t.Errorf("Expected reason to be 'Adapter(DataStore):Recognized', got '%s'", gate.EvaluationDetails.Reason)
	}

	if !gate.Value {
		t.Error("Expected gate value to be true")
	}

	if gate.EvaluationDetails.Lcut != 1729873603830 {
		t.Errorf("Expected lcut to be 1729873603830, got %d", gate.EvaluationDetails.Lcut)
	}

	if dataStore.get_called_count <= 1 {
		t.Errorf("Expected get_called_count to be > 1, got %d", dataStore.get_called_count)
	}
}

// Test data store usage with set operations
func TestDataStoreUsageSet(t *testing.T) {
	originalData := LoadTestData("eval_proj_dcs.json")
	updatedDataBytes := updateDCSContent(originalData, t)

	scrapi := serverSetup("eval_proj_dcs.json", &updatedDataBytes)

	dataStore := NewMockDataStore("test_param")
	dataStore.should_poll = false

	options := statsig.NewStatsigOptionsBuilder().
		WithDataStore(dataStore).
		WithSpecsUrl(scrapi.GetUrlForEndpoint("/v2/download_config_specs")).
		WithLogEventUrl(scrapi.GetUrlForEndpoint("/v1/log_event")).
		WithSpecsSyncIntervalMs(1000).
		WithOutputLogLevel("DEBUG").
		Build()

	statsigInstance, _ := statsig.NewStatsig("secret-key", *options)
	user := statsig.NewStatsigUserBuilder().WithUserID("test_user_id").Build()

	_, err := statsigInstance.Initialize()
	if err != nil {
		t.Fatalf("Failed to initialize Statsig: %v", err)
	}
	defer statsigInstance.Shutdown()

	if !dataStore.init_called {
		t.Error("Expected data store initialize to be called")
	}

	gate := statsigInstance.GetFeatureGate(*user, "test_public", nil)

	if gate.EvaluationDetails.Reason != "Adapter(DataStore):Recognized" {
		t.Errorf("Expected reason to be 'Adapter(DataStore):Recognized', got '%s'", gate.EvaluationDetails.Reason)
	}

	time.Sleep(5 * time.Second)

	gateAfter := statsigInstance.GetFeatureGate(*user, "test_public", nil)
	statsigInstance.FlushEvents()

	if gateAfter.EvaluationDetails.Reason != "Network:Recognized" {
		t.Errorf("Expected reason to be 'Network:Recognized', got '%s'", gateAfter.EvaluationDetails.Reason)
	}

	if dataStore.content_set == "" {
		t.Error("Expected content_set to not be empty")
	}

	if dataStore.content_set != string(updatedDataBytes) {
		t.Errorf("Expected content_set to match updated DCS data")
	}

	var contentSetMap map[string]interface{}
	var updatedDataMap map[string]interface{}

	if err := json.Unmarshal([]byte(dataStore.content_set), &contentSetMap); err != nil {
		t.Errorf("Failed to parse content_set: %v", err)
	}
	if err := json.Unmarshal(updatedDataBytes, &updatedDataMap); err != nil {
		t.Errorf("Failed to parse updatedDataBytes: %v", err)
	}

	if !reflect.DeepEqual(contentSetMap, updatedDataMap) {
		t.Errorf("Expected contentSetMap to match updatedDataMap")
	}
}
