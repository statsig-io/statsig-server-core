package statsig_go_core

import (
	"encoding/json"
	"fmt"
	"runtime"

	"github.com/statsig-io/statsig-go-core/internal"
)

type DataStoreFunctions struct {
	Initialize                     func()
	Shutdown                       func()
	Get                            func(key string) string
	Set                            func(key string, value string, time *uint64)
	ShouldBeUsedForQueryingUpdates func(key string) bool
}

type DataStore struct {
	functions DataStoreFunctions
	ref       uint64
}

func NewDataStore(functions DataStoreFunctions) *DataStore {
	store := &DataStore{
		functions: functions,
		ref:       0,
	}

	store.ref = GetFFI().data_store_create(
		store.functions.Initialize,
		store.functions.Shutdown,
		// Get
		func(argPtr *byte, argLength uint64) *byte {
			keyStr := internal.GoStringFromPointer(argPtr, argLength)
			if keyStr == nil {
				return nil
			}

			result := []byte(store.functions.Get(*keyStr))
			return &result[0]
		},
		// Set
		func(argPtr *byte, argLength uint64) {
			data, err := tryMarshalDataStoreSetArgs(argPtr, argLength)
			if err != nil {
				fmt.Println("Error marshalling DataStore 'set' args", err)
				return
			}

			keyStr := data.Key
			valueStr := data.Value
			time := data.Time
			store.functions.Set(keyStr, valueStr, time)
		},
		// ShouldBeUsedForQueryingUpdates
		func(argPtr *byte, argLength uint64) bool {
			keyStr := internal.GoStringFromPointer(argPtr, argLength)
			if keyStr == nil {
				return false
			}

			return store.functions.ShouldBeUsedForQueryingUpdates(*keyStr)
		},
	)

	runtime.SetFinalizer(store, func(obj *DataStore) {
		GetFFI().data_store_release(obj.ref)
	})

	return store
}

func (d *DataStore) INTERNAL_testDataStore(path string, value string) string {
	return GetFFI().__internal__test_data_store(d.ref, path, value)
}

type dataStoreSetArgs struct {
	Key   string  `json:"key"`
	Value string  `json:"value"`
	Time  *uint64 `json:"time"`
}

func tryMarshalDataStoreSetArgs(inputPtr *byte, inputLength uint64) (*dataStoreSetArgs, error) {
	data := internal.GoStringFromPointer(inputPtr, inputLength)

	var args dataStoreSetArgs
	err := json.Unmarshal([]byte(*data), &args)
	if err != nil {
		return nil, err
	}

	return &args, nil
}
