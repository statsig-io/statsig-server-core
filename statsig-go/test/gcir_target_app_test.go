package test

import (
	"encoding/json"
	"os"
	"slices"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

type gcirScope struct {
	gates       []string
	configs     []string
	layers      []string
	paramStores []string
}

var appAScope = gcirScope{
	gates:       []string{"gate_app_a"},
	configs:     []string{"config_app_a", "exp_app_a", "cmab_app_a"},
	layers:      []string{"layer_app_a"},
	paramStores: []string{"ps_app_a"},
}

var appBScope = gcirScope{
	gates:       []string{"gate_app_b"},
	configs:     []string{"config_app_b", "exp_app_b", "cmab_app_b"},
	layers:      []string{"layer_app_b"},
	paramStores: []string{"ps_app_b"},
}

func TestGCIRTargetAppScoping(t *testing.T) {
	superKey := "client-super-key" // maps to "app_a" in the fixture
	appB := "app_b"

	cases := []struct {
		name     string
		options  statsig_go.ClientInitResponseOptions
		expected gcirScope
	}{
		{
			name:     "client sdk key alone scopes to mapped app",
			options:  statsig_go.ClientInitResponseOptions{ClientSDKKey: &superKey},
			expected: appAScope,
		},
		{
			name:     "target app id overrides key-derived app",
			options:  statsig_go.ClientInitResponseOptions{ClientSDKKey: &superKey, TargetAppID: &appB},
			expected: appBScope,
		},
		{
			name:     "target app id without client sdk key still filters",
			options:  statsig_go.ClientInitResponseOptions{TargetAppID: &appB},
			expected: appBScope,
		},
	}

	dcsData, err := os.ReadFile("../../statsig-rust/tests/data/dcs_target_app_filtering.json")
	if err != nil {
		t.Fatalf("error reading fixture: %v", err)
	}

	hashAlgo := "none"
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			statsig, _, user := SetupTestWithDcsData(t, dcsData)
			tc.options.HashAlgorithm = &hashAlgo

			rawResponse := statsig.GetClientInitResponseWithOptions(user, &tc.options)
			statsig.Shutdown()

			if rawResponse == nil {
				t.Fatal("response is nil")
			}

			var response map[string]any
			if err := json.Unmarshal([]byte(*rawResponse), &response); err != nil {
				t.Fatalf("error unmarshalling client init response: %v", err)
			}

			assertSectionKeys(t, response, "feature_gates", tc.expected.gates)
			assertSectionKeys(t, response, "dynamic_configs", tc.expected.configs)
			assertSectionKeys(t, response, "layer_configs", tc.expected.layers)
			assertSectionKeys(t, response, "param_stores", tc.expected.paramStores)
		})
	}
}

func assertSectionKeys(t *testing.T, response map[string]any, section string, expected []string) {
	t.Helper()

	obj, ok := response[section].(map[string]any)
	if !ok {
		t.Errorf("%s section missing from response", section)
		return
	}

	actual := make([]string, 0, len(obj))
	for name := range obj {
		actual = append(actual, name)
	}
	slices.Sort(actual)

	expected = slices.Clone(expected)
	slices.Sort(expected)

	if !slices.Equal(actual, expected) {
		t.Errorf("%s: got %v, want %v", section, actual, expected)
	}
}
