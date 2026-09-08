package statsig_go_core

import (
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

// This test lives in the package rather than in ./test because counting FFI
// round trips means swapping an unexported function pointer on the FFI struct.
func TestParameterStoreContainsMakesNoFFICalls(t *testing.T) {
	statsig, user := setupStatsigForFFICounting(t)
	defer statsig.Shutdown()

	store := statsig.GetParameterStore(user, "test_parameter_store")
	if !store.Contains("bool_param") {
		t.Fatalf("expected the store to contain bool_param, got %v", store.GetParameterList())
	}

	ffi := GetFFI()
	original := ffi.statsig_get_parameter_names_from_store
	defer func() { ffi.statsig_get_parameter_names_from_store = original }()

	calls := 0
	ffi.statsig_get_parameter_names_from_store = func(uint64, uint64, string, *uint64) *byte {
		calls++
		return nil
	}

	for i := range 5 {
		if !store.Contains("bool_param") {
			t.Errorf("bool_param should stay present on lookup %d", i)
		}
		if store.Contains("missing_param") {
			t.Errorf("missing_param should stay absent on lookup %d", i)
		}
		if len(store.GetParameterList()) != 1 {
			t.Errorf("parameter list should stay [bool_param] on lookup %d, got %v", i, store.GetParameterList())
		}
	}

	if calls != 0 {
		t.Errorf("expected no FFI calls after the store was fetched, got %d", calls)
	}
}

func setupStatsigForFFICounting(t *testing.T) (*Statsig, *StatsigUser) {
	dcs, err := os.ReadFile("../statsig-rust/tests/data/eval_proj_dcs.json")
	if err != nil {
		t.Fatalf("error reading dcs data: %v", err)
	}

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if strings.HasPrefix(r.URL.Path, "/v2/download_config_specs") {
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write(dcs)
			return
		}
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"success": true}`))
	}))
	t.Cleanup(srv.Close)

	opts, err := NewOptionsBuilder().
		WithSpecsUrl(srv.URL + "/v2/download_config_specs").
		WithLogEventUrl(srv.URL + "/v1/log_event").
		Build()
	if err != nil {
		t.Fatalf("error creating StatsigOptions: %v", err)
	}

	user, err := NewUserBuilderWithUserID("user-id").Build()
	if err != nil {
		t.Fatalf("error creating StatsigUser: %v", err)
	}

	statsig, err := NewStatsigWithOptions("secret-123", opts)
	if err != nil {
		t.Fatalf("error creating Statsig: %v", err)
	}
	statsig.Initialize()

	return statsig, user
}
