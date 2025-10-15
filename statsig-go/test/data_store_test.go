package test

import (
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

type SetCall struct {
	key   string
	value string
	time  *uint64
}

type MockDataStore struct {
	initializeCalled                   bool
	shutdownCalled                     bool
	getCall                            *string
	setCall                            *SetCall
	shouldBeUsedForQueryingUpdatesCall *string
}

func (m *MockDataStore) Initialize() {
	m.initializeCalled = true
}

func (m *MockDataStore) Shutdown() {
	m.shutdownCalled = true
}

func (m *MockDataStore) Get(key string) string {
	m.getCall = &key
	return "{\"result\": \"test\", \"time\": 1234567890}"
}

func (m *MockDataStore) Set(key string, value string, time *uint64) {
	m.setCall = &SetCall{
		key:   key,
		value: value,
		time:  time,
	}
}

func (m *MockDataStore) ShouldBeUsedForQueryingUpdates(key string) bool {
	m.shouldBeUsedForQueryingUpdatesCall = &key
	return false
}

func (m *MockDataStore) GetFunctions() statsig_go.DataStoreFunctions {
	return statsig_go.DataStoreFunctions{
		Initialize:                     m.Initialize,
		Shutdown:                       m.Shutdown,
		Get:                            m.Get,
		Set:                            m.Set,
		ShouldBeUsedForQueryingUpdates: m.ShouldBeUsedForQueryingUpdates,
	}
}

func TestDataStore(t *testing.T) {
	mockDataStore := &MockDataStore{}

	store := statsig_go.NewDataStore(mockDataStore.GetFunctions())

	store.INTERNAL_testDataStore("/v2/download_config_specs", "test")

	if !mockDataStore.initializeCalled {
		t.Error("Expected initialize to be called")
	}

	if *mockDataStore.getCall != "/v2/download_config_specs" {
		t.Error("Expected get to be called with /v2/download_config_specs")
	}

	setCall := mockDataStore.setCall
	if setCall == nil {
		t.Error("Expected set to be called")
	} else {
		if setCall.key != "/v2/download_config_specs" {
			t.Error("Expected set to be called with /v2/download_config_specs")
		}
		if setCall.value != "test" {
			t.Error("Expected set to be called with test")
		}
		if *setCall.time != 123 {
			t.Error("Expected set to be called with time")
		}
	}

	if *mockDataStore.shouldBeUsedForQueryingUpdatesCall != "/v2/download_config_specs" {
		t.Error("Expected shouldBeUsedForQueryingUpdates to be called with /v2/download_config_specs")
	}

	if !mockDataStore.shutdownCalled {
		t.Error("Expected shutdown to be called")
	}
}
