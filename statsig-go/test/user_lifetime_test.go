package test

import (
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

// A StatsigUser that is unreachable after an SDK call starts can be finalized
// while Rust is still evaluating it. Rust then cannot resolve the user and the
// call returns an empty result. The race is rare (about 1 in 2M calls under
// constant GC), so this test is probabilistic: it catches regressions but a
// pass does not prove the race is gone.
func TestUserIsKeptAliveDuringEvaluation(t *testing.T) {
	if testing.Short() {
		t.Skip("GC stress test")
	}

	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	const workers = 32
	const duration = 5 * time.Second
	opts := &statsig_go.DynamicConfigEvaluationOptions{DisableExposureLogging: true}

	var stop atomic.Bool
	var calls, lost atomic.Int64

	go func() {
		for !stop.Load() {
			runtime.GC()
		}
	}()

	var wg sync.WaitGroup
	for w := range workers {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for i := 0; !stop.Load(); i++ {
				user, err := statsig_go.NewUserBuilderWithUserID(fmt.Sprintf("user-%d-%d", id, i)).Build()
				if err != nil {
					t.Errorf("failed to build user: %v", err)
					return
				}

				config := statsig.GetDynamicConfigWithOptions(user, "test_experiment_no_targeting", opts)
				calls.Add(1)
				if config.EvaluationDetails.Reason == "" {
					lost.Add(1)
				}
			}
		}(w)
	}

	time.Sleep(duration)
	stop.Store(true)
	wg.Wait()

	if lost.Load() > 0 {
		t.Errorf("%d of %d evaluations lost their user mid-call", lost.Load(), calls.Load())
	}
}

func TestParameterStoreOutlivesUser(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	store := func() statsig_go.ParameterStore {
		user, err := statsig_go.NewUserBuilderWithUserID("a-user").Build()
		if err != nil {
			t.Fatalf("failed to build user: %v", err)
		}
		return statsig.GetParameterStore(user, "test_parameter_store")
	}()

	// Give the user's finalizer a chance to run if nothing keeps the user alive.
	for range 5 {
		runtime.GC()
		time.Sleep(10 * time.Millisecond)
	}

	if got := store.GetBool("bool_param", true); got != false {
		t.Errorf("GetBool(bool_param) = %v, want configured value false", got)
	}
	if got := store.GetString("missing_param", "fallback"); got != "fallback" {
		t.Errorf("GetString(missing_param) = %q, want fallback", got)
	}
}
