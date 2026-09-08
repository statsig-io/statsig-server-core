package test

import (
	"reflect"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestParameterStoreEvaluation(t *testing.T) {
	statsig, _, user := SetupTest(t)

	store := statsig.GetParameterStore(user, "test_parameter_store")
	if store.Name != "test_parameter_store" {
		t.Errorf("Parameter store name mismatch, got '%s'", store.Name)
	}

	boolVal := store.GetBool("bool_param", true)
	if boolVal != false {
		t.Errorf("Parameter store bool_param is not correct, got '%v'", boolVal)
	}

	missingVal := store.GetString("missing_param", "fallback")
	if missingVal != "fallback" {
		t.Errorf("Missing param should return fallback, got '%s'", missingVal)
	}

	missingArray := store.GetSlice("missing_array_param", []any{"fallback"})
	if len(missingArray) != 1 || missingArray[0] != "fallback" {
		t.Errorf("Missing array param should return fallback, got %v", missingArray)
	}

	missingObject := store.GetMap("missing_object_param", map[string]any{"fallback": "value"})
	if len(missingObject) != 1 || missingObject["fallback"] != "value" {
		t.Errorf("Missing object param should return fallback, got %v", missingObject)
	}

	statsig.Shutdown()
}

func TestParameterStoreContains(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	store := statsig.GetParameterStore(user, "test_parameter_store")
	if !store.Contains("bool_param") {
		t.Errorf("Parameter store should contain bool_param, got %v", store.GetParameterList())
	}
	if store.Contains("missing_param") {
		t.Errorf("Parameter store should not contain missing_param")
	}

	unknown := statsig.GetParameterStore(user, "not_a_real_store")
	if unknown.Contains("bool_param") {
		t.Errorf("Unknown parameter store should not contain any parameter")
	}
}

func TestParameterStoreGetParameterList(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	store := statsig.GetParameterStore(user, "test_parameter_store")
	names := store.GetParameterList()
	if !reflect.DeepEqual(names, []string{"bool_param"}) {
		t.Errorf("Parameter store list should be [bool_param], got %v", names)
	}

	unknown := statsig.GetParameterStore(user, "not_a_real_store")
	unknownNames := unknown.GetParameterList()
	if unknownNames == nil {
		t.Errorf("Unknown parameter store list should be an empty slice, got nil")
	}
	if len(unknownNames) != 0 {
		t.Errorf("Unknown parameter store list should be empty, got %v", unknownNames)
	}
}

// Every other method on ParameterStore tolerates a nil receiver by returning the
// fallback, so Contains and GetParameterList hold the same invariant.
func TestParameterStoreNilReceiver(t *testing.T) {
	var store *statsig_go.ParameterStore

	if store.Contains("bool_param") {
		t.Errorf("a nil parameter store should not contain anything")
	}

	names := store.GetParameterList()
	if names == nil {
		t.Errorf("a nil parameter store should return an empty slice, got nil")
	}
	if len(names) != 0 {
		t.Errorf("a nil parameter store should have no parameter names, got %v", names)
	}
}

// A nil user cannot be evaluated, so the store degrades to its fallbacks and to
// an empty parameter list instead of panicking. The degradation is logged once,
// when the store is fetched.
func TestParameterStoreWithNilUser(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	store := statsig.GetParameterStore(nil, "test_parameter_store")
	if store.Name != "test_parameter_store" {
		t.Errorf("Parameter store name mismatch, got '%s'", store.Name)
	}

	if store.Contains("bool_param") {
		t.Errorf("a store fetched with a nil user should report no parameters")
	}

	names := store.GetParameterList()
	if names == nil {
		t.Errorf("a store fetched with a nil user should return an empty slice, got nil")
	}
	if len(names) != 0 {
		t.Errorf("a store fetched with a nil user should have no parameter names, got %v", names)
	}

	if store.GetBool("bool_param", true) != true {
		t.Errorf("a store fetched with a nil user should return the fallback")
	}
}

// A configured value that happens to equal the caller's fallback is
// indistinguishable from a missing parameter through the getters alone. Contains
// is the only way to tell the two apart.
func TestParameterStoreContainsDistinguishesConfiguredValueFromFallback(t *testing.T) {
	statsig, _, user := SetupTest(t)
	defer statsig.Shutdown()

	store := statsig.GetParameterStore(user, "test_parameter_store")

	// bool_param is configured as false, which is also the fallback here.
	if store.GetBool("bool_param", false) != false {
		t.Errorf("bool_param should evaluate to false")
	}
	if !store.Contains("bool_param") {
		t.Errorf("bool_param is configured and should report as present")
	}

	if store.GetBool("missing_param", false) != false {
		t.Errorf("missing_param should fall back to false")
	}
	if store.Contains("missing_param") {
		t.Errorf("missing_param is not configured and should report as absent")
	}
}
