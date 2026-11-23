package statsig_go_core

import (
	"bytes"
	"encoding/json"
	"fmt"
	"math"
	"reflect"
	"strconv"
	"strings"
)

// StickyValuesV2 is a structured persistent assignment. SDK-decoded parameter
// numbers (including nested numbers) are json.Number, not float64. Evaluation
// results retain their existing numeric representation. Unknown record fields
// are ignored, as with StickyValues; this is not an opaque forwarder.
type StickyValuesV2 struct {
	Value                         bool                `json:"value"`
	JSONValue                     map[string]any      `json:"json_value"`
	RuleID                        string              `json:"rule_id"`
	GroupName                     *string             `json:"group_name"`
	SecondaryExposures            []SecondaryExposure `json:"secondary_exposures"`
	UndelegatedSecondaryExposures []SecondaryExposure `json:"undelegated_secondary_exposures"`
	ConfigDelegate                *string             `json:"config_delegate"`
	ExplicitParameters            *[]string           `json:"explicit_parameters"`
	Time                          int64               `json:"time"`
	ConfigVersion                 *int64              `json:"config_version,omitempty"`
}

// UserPersistedValuesV2 maps config names to assignments. Decode storage bytes
// directly into this type; decoding through map[string]any can round numbers.
type UserPersistedValuesV2 map[string]StickyValuesV2

type stickyValuesV2Wire StickyValuesV2

// UnmarshalJSON preserves numeric tokens and leaves the receiver unchanged on
// failure. Missing or null json_value means no parameter object, matching the
// native optional field; a present null parameter remains a present nil map entry.
func (v *StickyValuesV2) UnmarshalJSON(data []byte) error {
	if bytes.Equal(bytes.TrimSpace(data), []byte("null")) {
		return fmt.Errorf("persistent assignment must be an object")
	}
	var wire struct {
		stickyValuesV2Wire
		Value *bool `json:"value"`
	}
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	// json.Valid enforces exactly one complete JSON document, including when
	// this method is called directly rather than through json.Unmarshal.
	if !json.Valid(data) {
		return fmt.Errorf("invalid persistent assignment JSON")
	}
	if err := decoder.Decode(&wire); err != nil {
		return err
	}
	if wire.Value == nil {
		return fmt.Errorf("persistent assignment requires a non-null boolean value")
	}
	next := wire.stickyValuesV2Wire
	next.Value = *wire.Value
	if err := validateStickyValuesV2(StickyValuesV2(next)); err != nil {
		return err
	}
	*v = StickyValuesV2(next)
	return nil
}

// MarshalJSON rejects unsupported parameter values and integers outside the
// native i64/u64 range instead of silently converting them to floating point.
func (v StickyValuesV2) MarshalJSON() ([]byte, error) {
	// Native StickyValues requires a sequence for this field. Accept omitted,
	// null, and programmatically nil slices as no exposures, but always emit
	// an array. The value receiver keeps the caller's record unchanged.
	if v.SecondaryExposures == nil {
		v.SecondaryExposures = []SecondaryExposure{}
	}
	if err := validateStickyValuesV2(v); err != nil {
		return nil, err
	}
	data, err := json.Marshal(stickyValuesV2Wire(v))
	if err != nil {
		return nil, err
	}
	// Check the actual encoded tokens too: encoding/json can render a large
	// integral float as an integer token outside native i64/u64 bounds.
	var wire stickyValuesV2Wire
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err := decoder.Decode(&wire); err != nil {
		return nil, err
	}
	if err := validateStickyValuesV2(StickyValuesV2(wire)); err != nil {
		return nil, err
	}
	return data, nil
}

func validateStickyValuesV2(v StickyValuesV2) error {
	if v.Time < 0 {
		return fmt.Errorf("persistent assignment time must be nonnegative")
	}
	if v.ConfigVersion != nil && (*v.ConfigVersion < 0 || *v.ConfigVersion > math.MaxUint32) {
		return fmt.Errorf("persistent config_version exceeds native u32 range")
	}
	return validatePersistentJSON(reflect.ValueOf(v.JSONValue), 0)
}

func validatePersistentNumber(n json.Number) error {
	s := string(n)
	if s == "" || !json.Valid([]byte(s)) {
		return fmt.Errorf("invalid persistent number %q", s)
	}
	if strings.ContainsAny(s, ".eE") {
		if f, err := strconv.ParseFloat(s, 64); err != nil || math.IsInf(f, 0) || math.IsNaN(f) {
			return fmt.Errorf("persistent number outside native f64 range: %q", s)
		}
	} else if strings.HasPrefix(s, "-") {
		if _, err := strconv.ParseInt(s, 10, 64); err != nil {
			return fmt.Errorf("persistent integer outside native i64 range: %q", s)
		}
	} else if _, err := strconv.ParseUint(s, 10, 64); err != nil {
		return fmt.Errorf("persistent integer outside native u64 range: %q", s)
	}
	return nil
}

func validatePersistentJSON(v reflect.Value, depth int) error {
	if depth > 256 {
		return fmt.Errorf("persistent parameters contain a cycle or exceed depth 256")
	}
	if !v.IsValid() {
		return nil
	}
	if v.CanInterface() {
		if n, ok := v.Interface().(json.Number); ok {
			return validatePersistentNumber(n)
		}
	}
	switch v.Kind() {
	case reflect.Interface:
		if v.IsNil() {
			return nil
		}
		return validatePersistentJSON(v.Elem(), depth+1)
	case reflect.Map:
		if v.Type().Key().Kind() != reflect.String {
			return fmt.Errorf("persistent object keys must be strings")
		}
		it := v.MapRange()
		for it.Next() {
			if err := validatePersistentJSON(it.Value(), depth+1); err != nil {
				return err
			}
		}
	case reflect.Array, reflect.Slice:
		// []byte is base64-encoded by encoding/json, rather than a JSON array.
		if v.Type().Elem().Kind() == reflect.Uint8 {
			return fmt.Errorf("persistent parameters must use JSON arrays, not byte slices")
		}
		for i := 0; i < v.Len(); i++ {
			if err := validatePersistentJSON(v.Index(i), depth+1); err != nil {
				return err
			}
		}
	case reflect.Float32, reflect.Float64:
		f := v.Float()
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return fmt.Errorf("persistent parameters require finite numbers")
		}
	case reflect.String, reflect.Bool, reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64, reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
	default:
		return fmt.Errorf("unsupported persistent parameter type: %s", v.Type())
	}
	return nil
}

// PersistentStorageFunctionsV2 opts into structured JSON persistence. Callbacks
// have the same concurrency and lifetime requirements as the legacy adapter.
type PersistentStorageFunctionsV2 struct {
	Load   func(key string) *UserPersistedValuesV2
	Save   func(key string, configName string, data StickyValuesV2)
	Delete func(key string, configName string)
}

type persistentStorageArgsV2 struct {
	Key        string          `json:"key"`
	ConfigName string          `json:"config_name"`
	Data       *StickyValuesV2 `json:"data"`
}

// NewPersistentStorageV2 retains the existing native transport and storage
// lifetime. Existing constructors and callback semantics are unchanged.
func NewPersistentStorageV2(functions PersistentStorageFunctionsV2) *PersistentStorage {
	storage := &PersistentStorage{}
	return registerPersistentStorage(storage,
		func(ptr *byte, length uint64) *byte {
			key := consumePersistentStorageArgs(ptr, length)
			if key == nil || functions.Load == nil {
				return nil
			}
			result := functions.Load(*key)
			if result == nil {
				return nil
			}
			data, err := json.Marshal(*result)
			if err != nil {
				fmt.Println("Error marshalling typed persisted values", err)
				return nil
			}
			return storage.retainLoadBuffer(data)
		},
		persistentStorageV2SaveCallback(functions.Save, consumePersistentStorageArgs),
		func(ptr *byte, length uint64) {
			data := consumePersistentStorageArgs(ptr, length)
			if data == nil || functions.Delete == nil {
				return
			}
			var args struct {
				Key        string `json:"key"`
				ConfigName string `json:"config_name"`
			}
			if err := json.Unmarshal([]byte(*data), &args); err != nil {
				fmt.Println("Error decoding persistent delete", err)
				return
			}
			functions.Delete(args.Key, args.ConfigName)
		},
	)
}

// The injectable consumer permits malformed callback tests without invoking a
// native JSON test helper that unwraps input. Production uses the normal native
// copy/free helper before decoding, including on decode failure.
func persistentStorageV2SaveCallback(save func(string, string, StickyValuesV2), consume func(*byte, uint64) *string) func(*byte, uint64) {
	return func(ptr *byte, length uint64) {
		data := consume(ptr, length)
		if data == nil {
			return
		}
		var args persistentStorageArgsV2
		if err := json.Unmarshal([]byte(*data), &args); err != nil {
			fmt.Println("Error decoding typed persistent save", err)
			return
		}
		if args.Data == nil {
			return
		}
		if save != nil {
			save(args.Key, args.ConfigName, *args.Data)
		}
	}
}
