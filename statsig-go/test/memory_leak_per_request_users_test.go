package test

import (
	"fmt"
	"testing"
	"time"
)

// TestMemoryLeakPerRequestUsers simulates a server-side usage pattern where
// each incoming request carries a different end-user identity (e.g. a distinct
// accountUuid / sessionId / visitorId). The existing TestMemoryLeak allocates
// per-iteration users but discards them without querying — so any SDK state
// that's keyed on `userID` never gets exercised. This test passes the unique
// per-iteration user directly into GetFeatureGate / GetDynamicConfig /
// GetExperiment / GetLayer / GetClientInitResponse, which is what a real
// high-cardinality server workload looks like.
//
// Shares helpers (createUser, triggerGC, getRssBytes, humanizeBytes,
// loadLargeDcsData, SetupTestWithDcsData) with memory_leak_test.go.
func TestMemoryLeakPerRequestUsers(t *testing.T) {
	resData := loadLargeDcsData(t)
	statsig, scrapi, _ := SetupTestWithDcsData(t, resData)

	// Stop the mock from retaining every request body and every parsed
	// log_event payload. Without this, the bulk of measured RSS growth in
	// this test is the harness storing user payloads (especially the
	// ~100KB dummyString in createUser), not the SDK's own state.
	scrapi.DisableRecording()

	time.Sleep(1 * time.Second)

	// Warmup: let background threads / initial allocations settle.
	for i := range 10000 {
		u := createUser(t, i)
		_ = statsig.GetFeatureGate(u, "test_public")
		_ = statsig.GetDynamicConfig(u, "test_empty_array")
		_ = statsig.GetExperiment(u, "exp_with_obj_and_array")
		_ = statsig.GetLayer(u, "layer_with_many_params")
		_ = statsig.GetClientInitResponse(u)
	}

	time.Sleep(1 * time.Second)
	triggerGC()

	initialRss := getRssBytes(t)
	fmt.Println("Initial RSS after warmup: ", humanizeBytes(initialRss))

	// Hot loop: each iteration uses a fresh, unique userID. This is what
	// stresses any SDK state keyed on userID (exposure queue entries cloning
	// user objects, per-user evaluation caches, etc.).
	const iterations = 1280001
	for i := range iterations {
		if i == 10000 || i == 20000 || i == 40000 || i == 80000 || i == 160000 || i == 320000 || i == 640000 || i == 1280000 {
			triggerGC()
			rss := getRssBytes(t)
			fmt.Println("RSS at iteration ", i, ": ", humanizeBytes(rss))
		}
		u := createUser(t, i)
		_ = statsig.GetFeatureGate(u, "test_public")
		_ = statsig.GetDynamicConfig(u, "test_empty_array")
		_ = statsig.GetExperiment(u, "exp_with_obj_and_array")
		_ = statsig.GetLayer(u, "layer_with_many_params")
		_ = statsig.GetClientInitResponse(u)
	}

	time.Sleep(1 * time.Second)
	triggerGC()

	finalRss := getRssBytes(t)
	fmt.Println("Final RSS: ", humanizeBytes(finalRss))

	percentChange := float64(finalRss-initialRss) / float64(initialRss) * 100
	delta := finalRss - initialRss

	if percentChange > 50 {
		t.Errorf("Memory leak detected with per-request users: %s (%.2f%%)", humanizeBytes(delta), percentChange)
	} else {
		fmt.Printf("Memory change within acceptable range: %s (%.2f%%)", humanizeBytes(delta), percentChange)
	}
}
