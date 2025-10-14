package statsig_go_core

import (
	"C"
)
import (
	"encoding/json"
	"fmt"
	"runtime"
)

type PersistentStorageFunctions struct {
	Load   func(key string) string
	Save   func(key string, configName string, data StickyValues)
	Delete func(key string, configName string)
}

type PersistentStorage struct {
	functions PersistentStorageFunctions
	ref       uint64
}

type SecondaryExposure struct {
	Gate      string `json:"gate"`
	GateValue string `json:"gateValue"`
	RuleID    string `json:"ruleID"`
}

type StickyValues struct {
	Value                         bool                `json:"value"`
	JSONValue                     map[string]string   `json:"json_value"`
	RuleID                        string              `json:"rule_id"`
	GroupName                     *string             `json:"group_name"`
	SecondaryExposures            []SecondaryExposure `json:"secondary_exposures"`
	UndelegatedSecondaryExposures []SecondaryExposure `json:"undelegated_secondary_exposures"`
	ConfigDelegate                *string             `json:"config_delegate"`
	ExplicitParameters            *[]string           `json:"explicit_parameters"`
	Time                          int64               `json:"time"`
	ConfigVersion                 *int64              `json:"config_version,omitempty"`
}

type persistentStorageArgs struct {
	Key        string        `json:"key"`
	ConfigName string        `json:"config_name"`
	Data       *StickyValues `json:"data,omitempty"`
}

func NewPersistentStorage(functions PersistentStorageFunctions) *PersistentStorage {
	storage := &PersistentStorage{
		functions: functions,
		ref:       0,
	}

	storage.ref = GetFFI().persistent_storage_create(
		// Load
		func(key *C.char) *C.char {
			keyStr := C.GoString(key)
			result := storage.functions.Load(keyStr)
			return C.CString(result)
		},
		// Save
		func(args *C.char) {
			data, err := tryMarshalPersistentStorageArgs(args)
			if err != nil {
				fmt.Println("Error marshalling persistent storage args", err)
				return
			}

			if data.Data == nil {
				fmt.Println("Error marshalling persistent storage args: Data is nil")
				return
			}

			storage.functions.Save(data.Key, data.ConfigName, *data.Data)
		},
		// Delete
		func(args *C.char) {
			data, err := tryMarshalPersistentStorageArgs(args)
			if err != nil {
				fmt.Println("Error marshalling persistent storage args", err)
				return
			}
			storage.functions.Delete(data.Key, data.ConfigName)
		},
	)

	runtime.SetFinalizer(storage, func(obj *PersistentStorage) {
		GetFFI().persistent_storage_release(obj.ref)
	})

	return storage
}

func (c *PersistentStorage) INTERNAL_testPersistentStorage(action string, key string, configName string, data string) *string {
	result := GetFFI().__internal__test_persistent_storage(c.ref, action, key, configName, data)

	if result == nil {
		return nil
	}

	goString := C.GoString(result)
	return &goString

}

func tryMarshalPersistentStorageArgs(input *C.char) (*persistentStorageArgs, error) {
	data := C.GoString(input)

	var args persistentStorageArgs
	err := json.Unmarshal([]byte(data), &args)
	if err != nil {
		fmt.Println("Error unmarshalling persistent storage args", err)
		return nil, err
	}

	return &args, nil
}
