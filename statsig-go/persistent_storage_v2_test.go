package statsig_go_core

import (
	"encoding/json"
	"math"
	"reflect"
	"strings"
	"sync"
	"testing"
)

func TestPersistentV2DecodeContract(t *testing.T) {
	raw := `{"value":true,"json_value":{"s":"","b":true,"a":[null,9007199254740993,{"n":18446744073709551615}],"o":{},"nil":null,"empty":[]},"rule_id":"r","secondary_exposures":[],"time":1}`
	var values UserPersistedValuesV2
	if err := json.Unmarshal([]byte(`{"exp":`+raw+`}`), &values); err != nil {
		t.Fatal(err)
	}
	v := values["exp"]
	if _, ok := v.JSONValue["absent"]; ok {
		t.Fatal("missing key appeared")
	}
	if x, ok := v.JSONValue["nil"]; !ok || x != nil {
		t.Fatal("null lost")
	}
	a := v.JSONValue["a"].([]any)
	if a[1] != json.Number("9007199254740993") || a[2].(map[string]any)["n"] != json.Number("18446744073709551615") {
		t.Fatalf("numbers rounded: %#v", a)
	}
	encoded, err := json.Marshal(values)
	if err != nil {
		t.Fatal(err)
	}
	var again UserPersistedValuesV2
	if err = json.Unmarshal(encoded, &again); err != nil || !reflect.DeepEqual(values, again) {
		t.Fatalf("roundtrip: %s %v", encoded, err)
	}
	// Legacy readers cannot decode a whole map containing a new typed record.
	var legacy UserPersistedValues
	if err = json.Unmarshal(encoded, &legacy); err == nil {
		t.Fatal("expected old reader incompatibility")
	}
	if err = json.Unmarshal([]byte(`{"exp":{"value":true,"json_value":{"s":"legacy"}}}`), &values); err != nil || values["exp"].JSONValue["s"] != "legacy" {
		t.Fatal(err)
	}
	for _, raw := range []string{`{"value":false}`, `{"value":true,"json_value":null}`} {
		var v StickyValuesV2
		if err := json.Unmarshal([]byte(raw), &v); err != nil || v.JSONValue != nil {
			t.Fatalf("optional json_value: %s %v", raw, err)
		}
	}
}

func TestPersistentV2NumbersAndInvalidValues(t *testing.T) {
	for _, s := range []string{"9007199254740991", "9007199254740992", "9007199254740993", "-9223372036854775808", "9223372036854775807", "18446744073709551615", "1.25", "1e0", "-0", "1.234567890123456789"} {
		var v StickyValuesV2
		if err := json.Unmarshal([]byte(`{"value":true,"json_value":{"n":`+s+`}}`), &v); err != nil {
			t.Fatalf("%s: %v", s, err)
		}
		if v.JSONValue["n"] != json.Number(s) {
			t.Fatalf("token lost: %s", s)
		}
	}
	for _, value := range []any{json.Number(""), json.Number("01"), json.Number("NaN"), json.Number("true"), json.Number("1e400"), float64(1e20), json.Number("18446744073709551616"), json.Number("-9223372036854775809"), math.Inf(1), math.NaN(), func() {}, make(chan int), []byte("bytes"), map[int]int{1: 2}} {
		if _, err := json.Marshal(StickyValuesV2{JSONValue: map[string]any{"nested": []any{value}}}); err == nil {
			t.Fatalf("accepted unsupported %T %v", value, value)
		}
	}
	cyclic := map[string]any{}
	cyclic["self"] = cyclic
	if _, err := json.Marshal(StickyValuesV2{JSONValue: cyclic}); err == nil {
		t.Fatal("accepted cycle")
	}
	for _, raw := range []string{`null`, `[]`, `{"value":true,"json_value":[]}`, `{"value":true,"json_value":true}`, `{"value":true,"json_value":{"n":1e400}}`, `{"value":true,"json_value":{"n":18446744073709551616}}`, `{"value":true,"time":9223372036854775808}`, `{"value":true,"time":-1}`, `{"value":true,"config_version":4294967296}`, `{"value":"yes"}`, `{`, `{} {}`, `{} garbage`} {
		before := StickyValuesV2{RuleID: "unchanged"}
		v := before
		if err := v.UnmarshalJSON([]byte(raw)); err == nil {
			t.Fatalf("accepted %s", raw)
		}
		if !reflect.DeepEqual(v, before) {
			t.Fatalf("partial assignment: %s", raw)
		}
	}
}

func TestPersistentV2SafeCallback(t *testing.T) {
	for _, raw := range []string{`{`, `{} {}`, `null`, `{}`, `{"data":null}`, `{"data":{}}`, `{"data":{"value":null}}`, `{"data":{"value":true,"json_value":[]}}`, `{"data":{"value":true,"time":9223372036854775808}}`} {
		consumed, saved := 0, 0
		callback := persistentStorageV2SaveCallback(func(string, string, StickyValuesV2) { saved++ }, func(*byte, uint64) *string { consumed++; return &raw })
		callback(nil, uint64(len(raw)))
		if consumed != 1 || saved != 0 {
			t.Fatalf("bad callback counts %d %d for %s", consumed, saved, raw)
		}
	}
	calls := 0
	payload := `{"key":"u:userID","config_name":"exp","data":{"value":true,"json_value":{"n":9007199254740993}}}`
	callback := persistentStorageV2SaveCallback(func(k, n string, v StickyValuesV2) {
		calls++
		if k != "u:userID" || n != "exp" || v.JSONValue["n"] != json.Number("9007199254740993") {
			t.Error("identity or value lost")
		}
	}, func(*byte, uint64) *string { return &payload })
	var wg sync.WaitGroup
	// Independent callbacks can decode concurrently; adapters own their locking.
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			persistentStorageV2SaveCallback(nil, func(*byte, uint64) *string { return &payload })(nil, 0)
		}()
	}
	wg.Wait()
	callback(nil, 0)
	if calls != 1 {
		t.Fatal(calls)
	}
	persistentStorageV2SaveCallback(nil, func(*byte, uint64) *string { return nil })(nil, 0)
}

func TestPersistentV2OptionsEncoding(t *testing.T) {
	var nilMap *UserPersistedValuesV2
	empty := UserPersistedValuesV2{}
	for _, v := range []any{(*ExperimentEvaluationOptions)(nil), (*ExperimentEvaluationOptionsV2)(nil), (*LayerEvaluationOptions)(nil), (*LayerEvaluationOptionsV2)(nil)} {
		s, err := marshalPersistentEvaluationOptions(v)
		if err != nil || s != "" {
			t.Fatalf("nil option %q %v", s, err)
		}
	}
	for _, v := range []any{ExperimentEvaluationOptionsV2{UserPersistedValues: nilMap}, LayerEvaluationOptionsV2{UserPersistedValues: nilMap}} {
		s, _ := marshalPersistentEvaluationOptions(v)
		if strings.Contains(s, "user_persisted_values") {
			t.Fatal(s)
		}
	}
	for _, v := range []any{ExperimentEvaluationOptionsV2{true, &empty, true, true}, LayerEvaluationOptionsV2{true, &empty, true, true}} {
		s, _ := marshalPersistentEvaluationOptions(v)
		for _, part := range []string{`"user_persisted_values":{}`, `"disable_exposure_logging":true`, `"enforce_overrides":true`, `"enforce_targeting":true`} {
			if !strings.Contains(s, part) {
				t.Fatal(s)
			}
		}
	}
	invalid := UserPersistedValuesV2{"exp": {JSONValue: map[string]any{"n": json.Number("1e400")}}}
	if _, err := marshalPersistentEvaluationOptions(&ExperimentEvaluationOptionsV2{UserPersistedValues: &invalid}); err == nil {
		t.Fatal("invalid value reaches native evaluation")
	}
}

func TestPersistentV2MalformedCallbackReleasesBeforeDecode(t *testing.T) {
	for _, raw := range []string{`{`, `{} {}`, `{"data":null}`, `{"data":{"value":true,"json_value":[]}}`, `{"key":"u","config_name":"e","data":{"value":true,"json_value":{"n":6}}}`} {
		input := append([]byte(raw), 0)
		released, saves := 0, 0
		consume := func(p *byte, n uint64) *string {
			return consumePersistentStorageArgsWithRelease(p, n, func(got *byte) {
				if got != &input[0] {
					t.Error("wrong release pointer")
				}
				released++
				for i := range input {
					input[i] = 'x'
				}
			})
		}
		callback := persistentStorageV2SaveCallback(func(string, string, StickyValuesV2) {
			if released != 1 {
				t.Error("decode preceded release")
			}
			saves++
		}, consume)
		callback(&input[0], uint64(len(raw)))
		want := 0
		if strings.Contains(raw, `"key"`) {
			want = 1
		}
		if released != 1 || saves != want {
			t.Fatalf("release=%d save=%d raw=%s", released, saves, raw)
		}
	}
	released := 0
	if consumePersistentStorageArgsWithRelease(nil, 0, func(*byte) { released++ }) != nil || released != 0 {
		t.Fatal("nil pointer released")
	}
}

func TestPersistentV2RequiredExposureArray(t *testing.T) {
	for _, raw := range []string{`{"value":true,"json_value":{"n":6},"time":1}`, `{"value":true,"json_value":{"n":6},"secondary_exposures":null,"time":1}`, `{"value":true,"json_value":{"n":6},"secondary_exposures":[],"time":1}`} {
		var v StickyValuesV2
		if err := json.Unmarshal([]byte(raw), &v); err != nil {
			t.Fatal(err)
		}
		wasNil := v.SecondaryExposures == nil
		encoded, err := json.Marshal(v)
		if err != nil {
			t.Fatal(err)
		}
		var wire map[string]json.RawMessage
		if err := json.Unmarshal(encoded, &wire); err != nil {
			t.Fatal(err)
		}
		if string(wire["secondary_exposures"]) != "[]" {
			t.Fatalf("required native array: %s", encoded)
		}
		if (v.SecondaryExposures == nil) != wasNil {
			t.Fatal("marshaling mutated caller's record")
		}
	}
	// Programmatically constructed records use the same normalization path.
	raw, err := json.Marshal(StickyValuesV2{Value: true, JSONValue: map[string]any{"n": json.Number("6")}, Time: 1})
	if err != nil || !strings.Contains(string(raw), `"secondary_exposures":[]`) {
		t.Fatalf("nil slice: %s %v", raw, err)
	}
}

func TestPersistentV2RequiresAssignmentValue(t *testing.T) {
	for _, invalid := range []string{`{}`, `{"rule_id":"partial"}`, `{"value":null}`} {
		raw := []byte(`{"sibling":{"value":true},"active_experiment":` + invalid + `}`)
		var values UserPersistedValuesV2
		if err := json.Unmarshal(raw, &values); err == nil {
			t.Errorf("accepted incomplete assignment in %s", raw)
		}
	}
	v := StickyValuesV2{Value: true}
	if err := json.Unmarshal([]byte(`{"value":false}`), &v); err != nil || v.Value {
		t.Fatalf("explicit false: value=%v err=%v", v.Value, err)
	}
}
