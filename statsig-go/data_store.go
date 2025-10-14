package statsig_go_core

import (
	"C"
)
import "runtime"

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
		func(key *C.char) *C.char {
			keyStr := C.GoString(key)
			result := store.functions.Get(keyStr)
			return C.CString(result)
		},
		// Set
		func(key *C.char, value *C.char, time *uint64) {
			keyStr := C.GoString(key)
			valueStr := C.GoString(value)
			store.functions.Set(keyStr, valueStr, time)
		},
		// ShouldBeUsedForQueryingUpdates
		func(key *C.char) bool {
			keyStr := C.GoString(key)
			return store.functions.ShouldBeUsedForQueryingUpdates(keyStr)
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
