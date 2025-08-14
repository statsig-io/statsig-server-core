package statsig

/*
#include "statsig_ffi.h"
#include <stdlib.h>

typedef void (*initialize_fn_t)(uint64_t);
typedef void (*shutdown_fn_t)(uint64_t);
typedef char* (*get_fn_t)(uint64_t, char*);
typedef void (*set_fn_t)(uint64_t, char*, char*, uint64_t);
typedef bool (*support_polling_fn_t)(uint64_t, char*);

void go_data_store_initialize(uint64_t dataStoreID);
void go_data_store_shutdown(uint64_t dataStoreID);
char* go_data_store_get(uint64_t dataStoreID, char* key);
void go_data_store_set(uint64_t dataStoreID, char* key, char* value, uint64_t time);
bool go_data_store_support_polling_updates_for(uint64_t dataStoreID, char* key);
*/
import "C"
import (
	"runtime"
	"sync"
)

type DataStoreInterface interface {
	Get(key string) string
	Set(key string, value string)
	Initialize()
	Shutdown()
	ShouldBeUsedForQueryingUpdates(key string) bool
}

// global registry to store data store implementations indexed by rust data store id
var (
	dataStoreRegistry      = make(map[uint64]DataStoreInterface)
	registryMutexDataStore sync.RWMutex
)

//export go_data_store_initialize
func go_data_store_initialize(dataStoreID C.uint64_t) {
	registryMutexDataStore.RLock()
	defer registryMutexDataStore.RUnlock()

	if dataStore, exists := dataStoreRegistry[uint64(dataStoreID)]; exists {
		dataStore.Initialize()
	}
}

//export go_data_store_shutdown
func go_data_store_shutdown(dataStoreID C.uint64_t) {
	registryMutexDataStore.RLock()
	defer registryMutexDataStore.RUnlock()

	if dataStore, exists := dataStoreRegistry[uint64(dataStoreID)]; exists {
		dataStore.Shutdown()
	}
}

//export go_data_store_get
func go_data_store_get(dataStoreID C.uint64_t, key *C.char) *C.char {
	registryMutexDataStore.RLock()
	defer registryMutexDataStore.RUnlock()

	if dataStore, exists := dataStoreRegistry[uint64(dataStoreID)]; exists {
		goKey := C.GoString(key)
		result := dataStore.Get(goKey)
		return C.CString(result)
	}
	return C.CString("")
}

//export go_data_store_set
func go_data_store_set(dataStoreID C.uint64_t, key *C.char, value *C.char, time C.uint64_t) {
	registryMutexDataStore.RLock()
	defer registryMutexDataStore.RUnlock()

	if dataStore, exists := dataStoreRegistry[uint64(dataStoreID)]; exists {
		goKey := C.GoString(key)
		goValue := C.GoString(value)
		dataStore.Set(goKey, goValue)
	}
}

//export go_data_store_support_polling_updates_for
func go_data_store_support_polling_updates_for(dataStoreID C.uint64_t, key *C.char) C.bool {
	registryMutexDataStore.RLock()
	defer registryMutexDataStore.RUnlock()

	if dataStore, exists := dataStoreRegistry[uint64(dataStoreID)]; exists {
		goKey := C.GoString(key)
		result := dataStore.ShouldBeUsedForQueryingUpdates(goKey)
		return C.bool(result)
	}
	return C.bool(false)
}

func NewDataStore(dataStore DataStoreInterface) uint64 {
	data_store_ref := C.data_store_create(
		(C.initialize_fn_t)(C.go_data_store_initialize),
		(C.shutdown_fn_t)(C.go_data_store_shutdown),
		(C.get_fn_t)(C.go_data_store_get),
		(C.set_fn_t)(C.go_data_store_set),
		(C.support_polling_fn_t)(C.go_data_store_support_polling_updates_for),
	)

	C.data_store_set_ref(data_store_ref)

	registryMutexDataStore.Lock()
	dataStoreRegistry[uint64(data_store_ref)] = dataStore
	registryMutexDataStore.Unlock()

	if impl_ptr, ok := dataStore.(interface{}); ok {
		runtime.SetFinalizer(impl_ptr, func(obj interface{}) {
			C.data_store_release(data_store_ref)
		})
	}

	return uint64(data_store_ref)
}
