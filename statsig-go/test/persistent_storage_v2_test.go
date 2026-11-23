package test

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"runtime"
	"strings"
	"sync"
	"testing"

	statsig "github.com/statsig-io/statsig-go-core"
)

func TestPersistentV2NativeTypes(t *testing.T) {
	for _, token := range []string{`""`, `"hello"`, `6`, `1.25`, `true`, `false`, `null`, `[]`, `{}`, `[1,"two",false,null]`, `{"n":9007199254740993}`, `9007199254740991`, `9007199254740992`, `9007199254740993`, `-9223372036854775808`, `9223372036854775807`, `18446744073709551615`, `1e0`, `-0`} {
		t.Run(token, func(t *testing.T) {
			var mu sync.Mutex
			var data []byte
			calls, deletes := 0, 0
			storage := statsig.NewPersistentStorageV2(statsig.PersistentStorageFunctionsV2{
				Load: func(key string) *statsig.UserPersistedValuesV2 {
					mu.Lock()
					defer mu.Unlock()
					if data == nil {
						return nil
					}
					var value statsig.StickyValuesV2
					if err := json.Unmarshal(data, &value); err != nil {
						t.Error(err)
						return nil
					}
					values := statsig.UserPersistedValuesV2{"exp": value}
					return &values
				},
				Save: func(key, name string, v statsig.StickyValuesV2) {
					mu.Lock()
					defer mu.Unlock()
					calls++
					if key != "u:userID" || name != "exp" {
						t.Errorf("wrong identity %s %s", key, name)
					}
					var err error
					data, err = json.Marshal(v)
					if err != nil {
						t.Error(err)
					}
				}, Delete: func(key, name string) {
					deletes++
					if key != "u:userID" || name != "exp" {
						t.Error("delete identity")
					}
				},
			})
			defer runtime.KeepAlive(storage)
			raw := fmt.Sprintf(`{"value":true,"json_value":{"parameter":%s,"nil":null},"rule_id":"r","group_name":"Treatment","secondary_exposures":[{"gate":"g","gateValue":"true","ruleID":"gr"}],"undelegated_secondary_exposures":[],"config_delegate":"delegate","explicit_parameters":["parameter"],"time":1800000000000,"config_version":1}`, token)
			storage.INTERNAL_testPersistentStorage("save", "u:userID", "exp", raw)
			if calls != 1 {
				t.Fatalf("Save count %d", calls)
			}
			var expected statsig.StickyValuesV2
			// Native may normalize float/exponent spelling; compare against its first
			// emitted record, then require subsequent storage transport to preserve it.
			if err := json.Unmarshal(data, &expected); err != nil {
				t.Fatal(err)
			}
			if expected.RuleID != "r" || expected.GroupName == nil || *expected.GroupName != "Treatment" || expected.Time != 1800000000000 || expected.ConfigVersion == nil || *expected.ConfigVersion != 1 || expected.ConfigDelegate == nil || *expected.ConfigDelegate != "delegate" || expected.ExplicitParameters == nil || !reflect.DeepEqual(*expected.ExplicitParameters, []string{"parameter"}) || len(expected.SecondaryExposures) != 1 || expected.SecondaryExposures[0].RuleID != "gr" {
				t.Fatalf("metadata lost: %#v", expected)
			}
			loadedJSON := storage.INTERNAL_testPersistentStorage("load", "u:userID", "", "")
			var loaded statsig.UserPersistedValuesV2
			if err := json.Unmarshal([]byte(loadedJSON), &loaded); err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(loaded["exp"], expected) {
				t.Fatalf("native load mismatch: %s", loadedJSON)
			}
			if _, ok := expected.JSONValue["absent"]; ok {
				t.Fatal("absent appeared")
			}
			if v, ok := expected.JSONValue["nil"]; !ok || v != nil {
				t.Fatal("null lost")
			}
			if token == `9007199254740993` || token == `-9223372036854775808` || token == `18446744073709551615` {
				if expected.JSONValue["parameter"] != json.Number(token) {
					t.Fatalf("precision lost %#v", expected.JSONValue)
				}
			}
			if token == `{"n":9007199254740993}` && expected.JSONValue["parameter"].(map[string]any)["n"] != json.Number("9007199254740993") {
				t.Fatal("nested precision lost")
			}
			storage.INTERNAL_testPersistentStorage("delete", "u:userID", "exp", "")
			if deletes != 1 {
				t.Fatal(deletes)
			}
		})
	}
}

func setupPersistentV2(t *testing.T, functions statsig.PersistentStorageFunctionsV2) (*statsig.Statsig, *statsig.PersistentStorage, *MockScrapi) {
	t.Helper()
	raw, err := os.ReadFile("data/enforce_sticky_dcs.json")
	if err != nil {
		t.Fatal(err)
	}
	var specs map[string]any
	d := json.NewDecoder(bytes.NewReader(raw))
	d.UseNumber()
	if err = d.Decode(&specs); err != nil {
		t.Fatal(err)
	}
	params := map[string]any{"s": "", "n": json.Number("9007199254740993"), "b": true, "array": []any{json.Number("2"), nil, "s"}, "object": map[string]any{"n": json.Number("18446744073709551615")}, "emptyObject": map[string]any{}, "emptyArray": []any{}, "nil": nil}
	exp := specs["dynamic_configs"].(map[string]any)["enforce_exp"].(map[string]any)
	exp["rules"].([]any)[2].(map[string]any)["returnValue"] = params
	names := []string{}
	for k := range params {
		names = append(names, k)
	}
	exp["explicitParameters"] = names
	raw, err = json.Marshal(specs)
	if err != nil {
		t.Fatal(err)
	}
	scrapi := NewMockScrapi()
	t.Cleanup(scrapi.Close)
	scrapi.Stub("GET", "/v2/download_config_specs/secret-123.json", StubResponse{Status: 200, Body: raw})
	scrapi.Stub("POST", "/v1/log_event", StubResponse{Status: 200, Body: []byte(`{"success":true}`)})
	storage := statsig.NewPersistentStorageV2(functions)
	t.Cleanup(func() { runtime.KeepAlive(storage) })
	opts, err := statsig.NewOptionsBuilder().WithSpecsUrl(scrapi.URL() + "/v2/download_config_specs").WithLogEventUrl(scrapi.URL() + "/v1/log_event").WithPersistentStorage(storage).Build()
	if err != nil {
		t.Fatal(err)
	}
	client, err := statsig.NewStatsigWithOptions("secret-123", opts)
	if err != nil {
		t.Fatal(err)
	}
	client.Initialize()
	t.Cleanup(client.Shutdown)
	return client, storage, scrapi
}

func TestPersistentV2EvaluationRoundTrip(t *testing.T) {
	for _, layer := range []bool{false, true} {
		t.Run(fmt.Sprintf("layer=%v", layer), func(t *testing.T) {
			var mu sync.Mutex
			rows := map[string][]byte{}
			saves, deletes := 0, 0
			name := "enforce_exp"
			if layer {
				name = "enforce_layer"
			}
			functions := statsig.PersistentStorageFunctionsV2{
				Load: func(key string) *statsig.UserPersistedValuesV2 {
					mu.Lock()
					defer mu.Unlock()
					if len(rows) == 0 {
						return nil
					}
					var record statsig.StickyValuesV2
					if err := json.Unmarshal(rows[name], &record); err != nil {
						t.Error(err)
						return nil
					}
					v := statsig.UserPersistedValuesV2{name: record}
					return &v
				},
				Save: func(key, config string, v statsig.StickyValuesV2) {
					mu.Lock()
					defer mu.Unlock()
					saves++
					if key != "synthetic-user:userID" || config != name {
						t.Errorf("identity %s %s", key, config)
					}
					b, err := json.Marshal(v)
					if err != nil {
						t.Error(err)
					}
					rows[config] = append([]byte(nil), b...)
				},
				Delete: func(string, string) { mu.Lock(); defer mu.Unlock(); deletes++ },
			}
			client, storage, scrapi := setupPersistentV2(t, functions)
			targeted := makeEnforceUser(t, "synthetic-user", true)
			empty := statsig.UserPersistedValuesV2{}
			firstRule := ""
			firstGroup := ""
			var firstValues map[string]any
			if !layer {
				first := client.GetExperimentWithOptionsV2(targeted, name, &statsig.ExperimentEvaluationOptionsV2{DisableExposureLogging: true, UserPersistedValues: &empty})
				firstRule = first.RuleID
				firstValues = first.Value
				if first.GroupName != nil {
					firstGroup = *first.GroupName
				}
				if first.GetNumber("n", -1) != float64(9007199254740993) {
					t.Fatal("numeric getter contract changed")
				}
			} else {
				first := client.GetLayerWithOptionsV2(targeted, name, &statsig.LayerEvaluationOptionsV2{DisableExposureLogging: true, UserPersistedValues: &empty})
				firstRule = first.RuleID
				if first.GroupName != nil {
					firstGroup = *first.GroupName
				}
				if !first.GetBool("b", false) || first.GetMap("object", nil) == nil || first.GetSlice("array", nil) == nil {
					t.Fatal("layer values missing")
				}
			}
			if saves != 1 || deletes != 0 {
				t.Fatalf("callbacks: save=%d delete=%d", saves, deletes)
			}
			// Only bytes survive the first evaluation: load through application decoder.
			loaded := functions.Load("synthetic-user:userID")
			if loaded == nil {
				t.Fatal("application load failed")
			}
			record := (*loaded)[name]
			if record.RuleID != firstRule || record.GroupName == nil || *record.GroupName != firstGroup || record.Time == 0 || record.ConfigVersion == nil || record.JSONValue["n"] != json.Number("9007199254740993") {
				t.Fatalf("stored assignment wrong: %#v", record)
			}
			if layer && (record.ConfigDelegate == nil || *record.ConfigDelegate != "enforce_exp" || record.ExplicitParameters == nil) {
				t.Fatalf("layer metadata wrong: %#v", record)
			}
			// Independently exercise native Load transport before evaluating typed options.
			native := storage.INTERNAL_testPersistentStorage("load", "synthetic-user:userID", "", "")
			var nativeLoaded statsig.UserPersistedValuesV2
			if err := json.Unmarshal([]byte(native), &nativeLoaded); err != nil || !reflect.DeepEqual(nativeLoaded, *loaded) {
				t.Fatalf("native Load: %s %v", native, err)
			}
			untargeted := makeEnforceUser(t, "synthetic-user", false)
			if !layer {
				control := client.GetExperimentWithOptionsV2(untargeted, name, &statsig.ExperimentEvaluationOptionsV2{DisableExposureLogging: true})
				if control.RuleID == firstRule {
					t.Fatal("control did not change")
				}
				restored := client.GetExperimentWithOptionsV2(untargeted, name, &statsig.ExperimentEvaluationOptionsV2{DisableExposureLogging: true, UserPersistedValues: loaded})
				if restored.RuleID != firstRule || !reflect.DeepEqual(restored.Value, firstValues) {
					t.Fatalf("experiment not restored: %#v", restored)
				}
			} else {
				control := client.GetLayerWithOptionsV2(untargeted, name, &statsig.LayerEvaluationOptionsV2{DisableExposureLogging: true})
				if control.RuleID == firstRule {
					t.Fatal("layer control did not change")
				}
				restored := client.GetLayerWithOptionsV2(untargeted, name, &statsig.LayerEvaluationOptionsV2{DisableExposureLogging: true, UserPersistedValues: loaded})
				if restored.RuleID != firstRule || restored.AllocatedExperimentName == nil || *restored.AllocatedExperimentName != "enforce_exp" || !restored.GetBool("b", false) || restored.GetNumber("n", -1) != float64(9007199254740993) || !restored.Contains("nil") || restored.Contains("absent") {
					t.Fatalf("layer not restored: %#v", restored)
				}
			}
			// The nonsticky control opts out and issues one expected Delete.
			if saves != 1 || deletes != 1 {
				t.Fatalf("restoration changed storage: saves=%d deletes=%d", saves, deletes)
			}
			client.FlushEvents()
			for _, event := range scrapi.Events() {
				if event["eventName"] == "statsig::config_exposure" || event["eventName"] == "statsig::layer_exposure" {
					t.Error("disabled exposure logged")
				}
			}
		})
	}
}

func TestPersistentV2EnforcementAndOptIn(t *testing.T) {
	saves, deletes := 0, 0
	client, _, _ := setupPersistentV2(t, statsig.PersistentStorageFunctionsV2{Load: func(string) *statsig.UserPersistedValuesV2 { return nil }, Save: func(string, string, statsig.StickyValuesV2) { saves++ }, Delete: func(string, string) { deletes++ }})
	user := makeEnforceUser(t, "plain-user", true)
	// nil map is opt-out; a nonnil empty map is opt-in.
	client.GetExperimentWithOptionsV2(user, "enforce_exp", nil)
	if saves != 0 {
		t.Fatal("nil options persisted")
	}
	empty := statsig.UserPersistedValuesV2{}
	client.GetExperimentWithOptionsV2(user, "enforce_exp", &statsig.ExperimentEvaluationOptionsV2{UserPersistedValues: &empty})
	if saves != 1 {
		t.Fatal("empty map did not opt in")
	}
	delegate := "enforce_exp"
	for _, layer := range []bool{false, true} {
		name := "enforce_exp"
		var d *string
		if layer {
			name = "enforce_layer"
			d = &delegate
		}
		group := "Sticky Group"
		persisted := statsig.UserPersistedValuesV2{name: {Value: true, JSONValue: map[string]any{"value": "sticky"}, RuleID: "sticky_rule_id", GroupName: &group, SecondaryExposures: []statsig.SecondaryExposure{}, ConfigDelegate: d, Time: 1700000000000}}
		override := makeEnforceUser(t, "override-user", true)
		untargeted := makeEnforceUser(t, "plain-user", false)
		for _, enforce := range []bool{false, true} {
			got := ""
			if layer {
				v := client.GetLayerWithOptionsV2(override, name, &statsig.LayerEvaluationOptionsV2{UserPersistedValues: &persisted, EnforceOverrides: enforce})
				got = v.GetString("value", "")
			} else {
				v := client.GetExperimentWithOptionsV2(override, name, &statsig.ExperimentEvaluationOptionsV2{UserPersistedValues: &persisted, EnforceOverrides: enforce})
				got = v.GetString("value", "")
			}
			want := "sticky"
			if enforce {
				want = "override_value"
			}
			if got != want {
				t.Fatalf("layer=%v enforceOverrides=%v got %s", layer, enforce, got)
			}
		}
		for _, enforce := range []bool{false, true} {
			got := ""
			if layer {
				v := client.GetLayerWithOptionsV2(untargeted, name, &statsig.LayerEvaluationOptionsV2{UserPersistedValues: &persisted, EnforceTargeting: enforce})
				got = v.GetString("value", "")
			} else {
				v := client.GetExperimentWithOptionsV2(untargeted, name, &statsig.ExperimentEvaluationOptionsV2{UserPersistedValues: &persisted, EnforceTargeting: enforce})
				got = v.GetString("value", "")
			}
			if (got == "sticky") == enforce {
				t.Fatalf("layer=%v enforceTargeting=%v got %s", layer, enforce, got)
			}
		}
	}
	if deletes == 0 {
		t.Fatal("expected Delete for opt-out/targeting enforcement")
	}
}

func TestPersistentV2ConcurrentCallbacksAndGC(t *testing.T) {
	var mu sync.Mutex
	loads, saves, deletes := 0, 0, 0
	v := statsig.StickyValuesV2{Value: true, JSONValue: map[string]any{"n": json.Number("9007199254740993")}, SecondaryExposures: []statsig.SecondaryExposure{}, Time: 1}
	raw, err := json.Marshal(v)
	if err != nil {
		t.Fatal(err)
	}
	storage := statsig.NewPersistentStorageV2(statsig.PersistentStorageFunctionsV2{
		Load: func(string) *statsig.UserPersistedValuesV2 {
			mu.Lock()
			loads++
			mu.Unlock()
			values := statsig.UserPersistedValuesV2{"e": v}
			return &values
		},
		Save:   func(string, string, statsig.StickyValuesV2) { mu.Lock(); saves++; mu.Unlock() },
		Delete: func(string, string) { mu.Lock(); deletes++; mu.Unlock() },
	})
	runtime.GC()
	var wg sync.WaitGroup
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for j := 0; j < 20; j++ {
				storage.INTERNAL_testPersistentStorage("save", "u", "e", string(raw))
				storage.INTERNAL_testPersistentStorage("load", "u", "", "")
				storage.INTERNAL_testPersistentStorage("delete", "u", "e", "")
			}
		}()
	}
	wg.Wait()
	runtime.GC()
	runtime.KeepAlive(storage)
	if loads != 160 || saves != 160 || deletes != 160 {
		t.Fatalf("callback counts %d %d %d", loads, saves, deletes)
	}
}

func TestPersistentV2RejectsInvalidLoad(t *testing.T) {
	invalid := statsig.UserPersistedValuesV2{"e": {JSONValue: map[string]any{"n": json.Number("1e400")}}}
	storage := statsig.NewPersistentStorageV2(statsig.PersistentStorageFunctionsV2{Load: func(string) *statsig.UserPersistedValuesV2 { return &invalid }})
	defer runtime.KeepAlive(storage)
	got := storage.INTERNAL_testPersistentStorage("load", "u", "", "")
	if got != "null" && got != "" {
		t.Fatalf("invalid load reached native: %s", got)
	}
}

func TestPersistentV2CallbacksDoNotLeakArgs(t *testing.T) {
	const iterations = 500
	bigKey := strings.Repeat("k", 200000)
	value := statsig.StickyValuesV2{Value: true, JSONValue: map[string]any{"text": strings.Repeat("v", 200000), "n": json.Number("9007199254740993")}, SecondaryExposures: []statsig.SecondaryExposure{}, Time: 1}
	data, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	loads := 0
	storage := statsig.NewPersistentStorageV2(statsig.PersistentStorageFunctionsV2{Load: func(string) *statsig.UserPersistedValuesV2 { loads++; return nil }, Save: func(string, string, statsig.StickyValuesV2) {}, Delete: func(string, string) {}})
	run := func(n int) {
		for i := 0; i < n; i++ {
			storage.INTERNAL_testPersistentStorage("load", bigKey, "", "")
			storage.INTERNAL_testPersistentStorage("save", bigKey, "e", string(data))
			storage.INTERNAL_testPersistentStorage("delete", bigKey, "e", "")
		}
	}
	run(10)
	triggerGC()
	initial := getRssBytes(t)
	run(iterations)
	triggerGC()
	final := getRssBytes(t)
	runtime.KeepAlive(storage)
	if loads != iterations+10 {
		t.Fatal(loads)
	}
	t.Logf("V2 RSS grew by %s (%s -> %s)", humanizeBytes(final-initial), humanizeBytes(initial), humanizeBytes(final))
	if final-initial > 100*1024*1024 {
		t.Fatal("V2 persistent callback args leak")
	}
}

// Regression for the independent review: a Go-accepted nil/omitted exposure
// slice must not invalidate the entire user map at the native boundary.
func TestPersistentV2MissingExposureNativeLoadAndEvaluation(t *testing.T) {
	for _, shape := range []string{"omitted", "null", "empty", "programmatic-nil"} {
		for _, layer := range []bool{false, true} {
			t.Run(fmt.Sprintf("%s/layer=%v", shape, layer), func(t *testing.T) {
				raw := `{"value":true,"json_value":{"n":6},"rule_id":"stored-rule","group_name":"Stored","time":1}`
				if shape == "null" {
					raw = strings.TrimSuffix(raw, "}") + `,"secondary_exposures":null}`
				}
				if shape == "empty" {
					raw = strings.TrimSuffix(raw, "}") + `,"secondary_exposures":[]}`
				}
				var record statsig.StickyValuesV2
				if shape == "programmatic-nil" {
					group := "Stored"
					record = statsig.StickyValuesV2{Value: true, JSONValue: map[string]any{"n": json.Number("6")}, RuleID: "stored-rule", GroupName: &group, Time: 1}
				} else if err := json.Unmarshal([]byte(raw), &record); err != nil {
					t.Fatal(err)
				}
				name := "enforce_exp"
				if layer {
					name = "enforce_layer"
					delegate := "enforce_exp"
					params := []string{"n"}
					record.ConfigDelegate = &delegate
					record.ExplicitParameters = &params
				}
				sibling := record
				sibling.SecondaryExposures = []statsig.SecondaryExposure{}
				sibling.RuleID = "sibling-rule"
				values := statsig.UserPersistedValuesV2{name: record, "valid-sibling": sibling}
				saves, deletes := 0, 0
				client, storage, _ := setupPersistentV2(t, statsig.PersistentStorageFunctionsV2{Load: func(string) *statsig.UserPersistedValuesV2 { return &values }, Save: func(string, string, statsig.StickyValuesV2) { saves++ }, Delete: func(string, string) { deletes++ }})
				native := storage.INTERNAL_testPersistentStorage("load", "u:userID", "", "")
				var loaded statsig.UserPersistedValuesV2
				if err := json.Unmarshal([]byte(native), &loaded); err != nil {
					t.Fatal(err)
				}
				if len(loaded) != 2 {
					t.Fatalf("native rejected user map (including valid sibling): %s", native)
				}
				if loaded[name].SecondaryExposures == nil || len(loaded[name].SecondaryExposures) != 0 || loaded["valid-sibling"].RuleID != "sibling-rule" || loaded[name].JSONValue["n"] != json.Number("6") {
					t.Fatalf("native load changed records: %s", native)
				}
				user := makeEnforceUser(t, "synthetic-user", false)
				// Original values exercise evaluation-options serialization independently
				// from the normalized map returned by native Load.
				if layer {
					v := client.GetLayerWithOptionsV2(user, name, &statsig.LayerEvaluationOptionsV2{UserPersistedValues: &values, DisableExposureLogging: true})
					if v.RuleID != "stored-rule" || v.GetNumber("n", -1) != 6 {
						t.Fatalf("layer lost assignment: %#v", v)
					}
				} else {
					v := client.GetExperimentWithOptionsV2(user, name, &statsig.ExperimentEvaluationOptionsV2{UserPersistedValues: &values, DisableExposureLogging: true})
					if v.RuleID != "stored-rule" || v.GetNumber("n", -1) != 6 {
						t.Fatalf("experiment lost assignment: %#v", v)
					}
				}
				if saves != 0 || deletes != 0 {
					t.Fatalf("unexpected callbacks: save=%d delete=%d", saves, deletes)
				}
			})
		}
	}
}
