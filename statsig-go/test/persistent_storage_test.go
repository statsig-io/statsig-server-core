package test

import (
	"encoding/json"
	"fmt"
	"testing"
	"time"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

type psRecordedCall struct {
	key        string
	configName string
	data       *statsig_go.StickyValues
}

type mockPersistentStorage struct {
	loadCall   *string
	saveCall   *psRecordedCall
	deleteCall *psRecordedCall
}

func (m *mockPersistentStorage) Load(key string) string {
	m.loadCall = &key

	json, err := json.Marshal(map[string]statsig_go.StickyValues{
		"test_load": createTestStickyValues(),
	})

	if err != nil {
		fmt.Println("Error marshalling sticky values", err)
		return ""
	}

	return string(json)
}

func createTestStickyValues() statsig_go.StickyValues {
	groupName := "test_group"
	configDelegate := "test_delegate"
	configVersion := int64(1)
	explicitParameters := []string{"test_param"}

	return statsig_go.StickyValues{
		Value: true,
		JSONValue: map[string]string{
			"header_text": "old user test",
		},
		RuleID:    "3ZCniK9rvnQyXDQlQ1tGD9",
		GroupName: &groupName,
		SecondaryExposures: []statsig_go.SecondaryExposure{
			{
				Gate:      "test_holdout",
				GateValue: "true",
				RuleID:    "default",
			},
		},
		UndelegatedSecondaryExposures: []statsig_go.SecondaryExposure{},
		ConfigVersion:                 &configVersion,
		Time:                          time.Now().Unix(),
		ConfigDelegate:                &configDelegate,
		ExplicitParameters:            &explicitParameters,
	}
}

func (m *mockPersistentStorage) Save(key string, configName string, data statsig_go.StickyValues) {
	m.saveCall = &psRecordedCall{
		key:        key,
		configName: configName,
		data:       &data,
	}
}

func (m *mockPersistentStorage) Delete(key string, configName string) {
	m.deleteCall = &psRecordedCall{
		key:        key,
		configName: configName,
	}
}

func (m *mockPersistentStorage) GetFunctions() statsig_go.PersistentStorageFunctions {
	return statsig_go.PersistentStorageFunctions{
		Load:   m.Load,
		Save:   m.Save,
		Delete: m.Delete,
	}
}

func TestPersistentStorageLoad(t *testing.T) {
	mock := &mockPersistentStorage{}

	client := statsig_go.NewPersistentStorage(mock.GetFunctions())

	result := client.INTERNAL_testPersistentStorage("load", "test_load", "", "")

	if *mock.loadCall != "test_load" {
		t.Error("loadCall should be test_load")
	}

	if result == nil {
		t.Error("result should be test_value")
		return
	}

	values := map[string]statsig_go.StickyValues{}
	err := json.Unmarshal([]byte(*result), &values)
	if err != nil {
		t.Error("Error unmarshalling sticky values", err)
	}

	testValues := values["test_load"]

	if testValues.Value != true {
		t.Error("value should be true")
	}

	if testValues.JSONValue["header_text"] != "old user test" {
		t.Error("json_value should be old user test")
	}

	if *testValues.GroupName != "test_group" {
		t.Error("group_name should be test_group")
	}

	if *testValues.ConfigDelegate != "test_delegate" {
		t.Error("config_delegate should be test_delegate")
	}

	if *testValues.ConfigVersion != 1 {
		t.Error("config_version should be 1")
	}

	if (*testValues.ExplicitParameters)[0] != "test_param" {
		t.Error("explicit_parameters should be test_param")
	}
}

func TestPersistentStorageSave(t *testing.T) {
	mock := &mockPersistentStorage{}

	client := statsig_go.NewPersistentStorage(mock.GetFunctions())

	testValues := createTestStickyValues()
	json, _ := json.Marshal(testValues)

	client.INTERNAL_testPersistentStorage("save", "test_save", "test_config", string(json))

	if mock.saveCall.data.RuleID != "3ZCniK9rvnQyXDQlQ1tGD9" {
		t.Error("rule_id should be 3ZCniK9rvnQyXDQlQ1tGD9")
	}

	if mock.saveCall.data.Value != true {
		t.Error("value should be true")
	}

	if mock.saveCall.data.JSONValue["header_text"] != "old user test" {
		t.Error("json_value should be old user test")
	}

	if *mock.saveCall.data.GroupName != "test_group" {
		t.Error("group_name should be test_group")
	}
}

func TestPersistentStorageDelete(t *testing.T) {
	mock := &mockPersistentStorage{}

	client := statsig_go.NewPersistentStorage(mock.GetFunctions())

	client.INTERNAL_testPersistentStorage("delete", "test_delete", "test_config", "")

	if mock.deleteCall.key != "test_delete" {
		t.Error("key should be test_delete")
	}

	if mock.deleteCall.configName != "test_config" {
		t.Error("config_name should be test_config")
	}
}
