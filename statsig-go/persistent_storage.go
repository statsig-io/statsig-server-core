package statsig_go_core

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync"

	"github.com/statsig-io/statsig-go-core/internal"
)

// The native side copies the buffer returned by the load callback (with
// CStr::from_ptr) immediately after the callback returns and never frees it.
// Without cgo we cannot hand back memory the native side can own, so we keep
// the most recent load buffers referenced from the storage itself: a buffer
// only becomes collectable once loadBufferRetention later loads have evicted
// it, which is long after the native side has copied it out.
const loadBufferRetention = 16

type PersistentStorageFunctions struct {
	Load   func(key string) *UserPersistedValues
	Save   func(key string, configName string, data StickyValues)
	Delete func(key string, configName string)
}

type PersistentStorage struct {
	functions PersistentStorageFunctions
	ref       uint64

	loadBufferMu    sync.Mutex
	loadBuffers     [loadBufferRetention][]byte
	loadBufferIndex int
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

type UserPersistedValues map[string]StickyValues

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
		"go",
		// Load
		func(argsPtr *byte, argsLength uint64) *byte {
			// The args payload is the raw storage key (not JSON).
			keyStr := consumePersistentStorageArgs(argsPtr, argsLength)
			if keyStr == nil {
				return nil
			}

			result := storage.functions.Load(*keyStr)

			if result == nil {
				return nil
			}

			json, err := json.Marshal(*result)
			if err != nil {
				fmt.Println("Error marshalling user persisted values", err)
				return nil
			}

			return storage.retainLoadBuffer(json)
		},
		// Save
		func(argsPtr *byte, argsLength uint64) {
			data, err := tryMarshalPersistentStorageArgs(argsPtr, argsLength)
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
		func(argsPtr *byte, argsLength uint64) {
			data, err := tryMarshalPersistentStorageArgs(argsPtr, argsLength)
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

func (c *PersistentStorage) INTERNAL_testPersistentStorage(action string, key string, configName string, data string) string {
	return GetFFI().__internal__test_persistent_storage(c.ref, action, key, configName, data)
}

// retainLoadBuffer returns a pointer to a NUL-terminated copy of data. The
// native side reads the result with CStr::from_ptr, so the buffer has to carry
// a terminator of its own rather than relying on whatever follows the Go slice.
// The buffer is kept referenced (see loadBufferRetention) so the GC cannot
// reclaim it while the native side is still copying out of it.
func (c *PersistentStorage) retainLoadBuffer(data []byte) *byte {
	buffer := make([]byte, len(data)+1)
	copy(buffer, data) // buffer[len(data)] stays 0

	c.loadBufferMu.Lock()
	c.loadBuffers[c.loadBufferIndex] = buffer
	c.loadBufferIndex = (c.loadBufferIndex + 1) % loadBufferRetention
	c.loadBufferMu.Unlock()

	return &buffer[0]
}

// consumePersistentStorageArgs takes ownership of the native-allocated args
// payload: it copies the bytes out using the explicit length (so embedded or
// missing NULs cannot truncate or over-read) and then frees the payload, which
// the C FFI hands to the callee.
func consumePersistentStorageArgs(inputPtr *byte, inputLength uint64) *string {
	if inputPtr == nil {
		return nil
	}

	data := internal.GoStringFromPointer(inputPtr, inputLength)
	GetFFI().free_string(inputPtr)

	return data
}

func tryMarshalPersistentStorageArgs(inputPtr *byte, inputLength uint64) (*persistentStorageArgs, error) {
	data := consumePersistentStorageArgs(inputPtr, inputLength)
	if data == nil {
		return nil, fmt.Errorf("persistent storage args pointer is nil")
	}

	var args persistentStorageArgs
	err := json.Unmarshal([]byte(*data), &args)
	if err != nil {
		fmt.Println("Error unmarshalling persistent storage args", err)
		return nil, err
	}

	return &args, nil
}
