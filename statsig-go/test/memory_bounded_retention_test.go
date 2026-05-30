// Memory leak test for the per-request-user pattern, Go binding.
//
// This is the Go counterpart of statsig-rust/tests/memory_leak_test.rs.
// It hits the full HTTP path through the FFI binding but uses a NON-RETAINING
// httptest.Server. The shared MockScrapi in this directory clones every
// request body into m.requests AND parses each log_event payload into
// m.events (see mock_scrapi.go handle()). Under high-volume per-user
// workloads that retention alone inflates RSS by GBs and is exactly what
// made PR #47's Go reproducer report "8.36 GB leak".
//
// What we're testing
// ------------------
// A real unbounded leak shows roughly *constant* per-iteration growth across
// checkpoints. The SDK's bounded caches (exposure dedupe set, hash memo
// cache, etc.) show per-iter growth that decays sharply as they fill toward
// their cap. We assert both:
//   1. Per-iter growth decays by at least 4x between the first and last
//      checkpoint (catches the "constant growth" leak signature).
//   2. Absolute RSS growth from baseline stays under a generous ceiling
//      (catches the "grows fast, plateaus high" failure mode).
//
// Running it
// ----------
//
//	go test -v -run TestPerRequestUsersMemoryIsBounded ./test/
//
// It is wrapped in testing.Short() short-circuit because:
//   - It depends on OS-specific RSS introspection (Linux + macOS only).
//   - It runs for ~30-60s and isn't useful on every CI commit.

package test

import (
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"runtime"
	"strings"
	"testing"
	"time"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

// nonRetainingServer stands up an httptest server that 200s every request
// without retaining any request data. The handler drains and discards the
// body, so neither request bodies nor event payloads accumulate in test
// memory. Compare with MockScrapi.handle() in mock_scrapi.go which appends
// to m.requests and m.events.
func nonRetainingServer(t *testing.T, dcsData []byte) *httptest.Server {
	t.Helper()
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Drain & drop. io.Discard never allocates beyond a small buffer.
		_, _ = io.Copy(io.Discard, r.Body)
		_ = r.Body.Close()

		if strings.Contains(r.URL.Path, "download_config_specs") {
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write(dcsData)
			return
		}
		// All other paths (log_event, get_id_lists, sdk_exception, ...)
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{}`))
	}))
	return srv
}

func newStatsigAgainst(t *testing.T, srvURL string) *statsig_go.Statsig {
	t.Helper()
	opts, err := statsig_go.NewOptionsBuilder().
		WithSpecsUrl(srvURL + "/v2/download_config_specs").
		WithLogEventUrl(srvURL + "/v1/log_event").
		WithEnvironment("development").
		WithDisableCountryLookup(true).
		WithDisableUserAgentParsing(true).
		WithOutputLogLevel("none").
		Build()
	if err != nil {
		t.Fatalf("error building options: %v", err)
	}

	statsig, err := statsig_go.NewStatsigWithOptions("secret-key", opts)
	if err != nil {
		t.Fatalf("error creating Statsig: %v", err)
	}
	statsig.Initialize()
	return statsig
}

// makeUser builds a minimal user — NO 100KB dummy custom string. The
// per-user payload in PR #47's createUser() was the single biggest source
// of inflated growth numbers. We keep this lean so any growth observed is
// proportional to what an actual server-side workload retains per userID.
func makeUser(t *testing.T, iter int) *statsig_go.StatsigUser {
	t.Helper()
	u, err := statsig_go.NewUserBuilderWithUserID(fmt.Sprintf("user_%d", iter)).
		WithEmail("user@example.com").
		WithIpAddress("127.0.0.1").
		WithLocale("en-US").
		WithAppVersion("1.0.0").
		WithCustom(map[string]any{"isAdmin": false}).
		Build()
	if err != nil {
		t.Fatalf("error creating StatsigUser: %v", err)
	}
	return u
}

func runIters(t *testing.T, statsig *statsig_go.Statsig, start, count int) {
	t.Helper()
	for i := 0; i < count; i++ {
		u := makeUser(t, start+i)
		_ = statsig.GetFeatureGate(u, "test_public")
		_ = statsig.GetDynamicConfig(u, "test_empty_array")
		_ = statsig.GetExperiment(u, "exp_with_obj_and_array")
		_ = statsig.GetLayer(u, "layer_with_many_params")
		_ = statsig.GetClientInitResponse(u)
		runtime.KeepAlive(u)
	}
}

// TestPerRequestUsersMemoryIsBounded is the proper version of PR #47's
// TestMemoryLeakPerRequestUsers. Same workload (unique userID per call,
// full evaluation surface) but no test-harness retention and a decay-based
// assertion that distinguishes a leak from bounded retention.
func TestPerRequestUsersMemoryIsBounded(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping long-running memory test in -short mode")
	}

	dcsData, err := os.ReadFile("../../statsig-rust/tests/data/eval_proj_dcs.json")
	if err != nil {
		t.Fatalf("error reading DCS fixture: %v", err)
	}

	srv := nonRetainingServer(t, dcsData)
	defer srv.Close()

	statsig := newStatsigAgainst(t, srv.URL)
	defer statsig.Shutdown()

	// Warmup: prime allocator + initial cache fills + cgo trampolines.
	runIters(t, statsig, 0, 200)
	time.Sleep(500 * time.Millisecond)
	triggerGC()

	baseline := getRssBytes(t)
	if baseline == 0 {
		t.Fatal("RSS introspection returned 0")
	}
	fmt.Println("Baseline RSS (after warmup):", humanizeBytes(baseline))

	// Sample at exponentially growing checkpoints so a constant-growth leak
	// shows up as a roughly constant per-iter delta across rows.
	checkpoints := []int{10_000, 20_000, 40_000, 80_000, 160_000, 320_000, 640_000, 1280_000}
	perIterSamples := make([]float64, 0, len(checkpoints))

	prevRSS := baseline
	prevIters := 0
	for _, target := range checkpoints {
		chunkIters := target - prevIters
		runIters(t, statsig, prevIters+200, chunkIters)
		time.Sleep(200 * time.Millisecond)
		triggerGC()

		rss := getRssBytes(t)
		chunkDelta := rss - prevRSS
		perIter := float64(chunkDelta) / float64(chunkIters)
		fmt.Printf(
			"After %6d iters: RSS=%10s  chunk_delta=%10s  per_iter=%8.1f B\n",
			target,
			humanizeBytes(rss),
			humanizeBytes(chunkDelta),
			perIter,
		)
		perIterSamples = append(perIterSamples, perIter)
		prevRSS = rss
		prevIters = target
	}

	firstChunk := perIterSamples[0]
	if firstChunk < 1 {
		firstChunk = 1
	}
	lastChunk := perIterSamples[len(perIterSamples)-1]
	decayRatio := lastChunk / firstChunk
	totalGrowth := prevRSS - baseline

	fmt.Println()
	fmt.Printf(
		"Per-iter growth: first_chunk=%.0f B  last_chunk=%.0f B  decay_ratio=%.3f\n",
		firstChunk, lastChunk, decayRatio,
	)
	fmt.Printf("Total RSS growth from baseline: %s\n", humanizeBytes(totalGrowth))

	// (1) Per-iter growth must decay at least 4x. A real unbounded leak
	// would show roughly flat per-iter growth (ratio ≈ 1.0); SDK-internal
	// bounded caches plateau as they fill toward their cap.
	if decayRatio >= 0.25 {
		t.Errorf(
			"per-iter growth did not decay enough — possible unbounded leak. "+
				"first_chunk_per_iter=%.1f B, last_chunk_per_iter=%.1f B, ratio=%.3f",
			firstChunk, lastChunk, decayRatio,
		)
	}

	// (2) Absolute ceiling. Catches the "grows fast, plateaus high" failure
	// mode where decay alone would pass. ~300 MB is generous for the
	// dedupe set (100k cap) + hash memo (10k cap) + event queue +
	// per-platform allocator slack across 80k unique users.
	const maxGrowthBytes int64 = 300 * 1024 * 1024
	if totalGrowth >= maxGrowthBytes {
		t.Errorf(
			"total RSS growth exceeded ceiling: %s > %s",
			humanizeBytes(totalGrowth),
			humanizeBytes(maxGrowthBytes),
		)
	}
}
