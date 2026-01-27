package test

import "testing"

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
