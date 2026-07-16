package test

import (
	"fmt"
	"sync"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

// Regression test for statsig-server-core issue #53 / ebitengine/purego#451.
// purego v0.9.x pooled its syscall arg structs, so simultaneous FFI calls
// could swap return-value pointers across goroutines. In this SDK that
// surfaced as double-frees in UseRustString and process-fatal SIGSEGVs in
// GoStringFromPointer. Each worker requests uniquely named entities and
// asserts the response echoes that name back; a swapped return shows up as
// a name mismatch, a garbage payload, or a hard crash of this test binary.
func TestConcurrentFFIStringReturnsAreNotSwapped(t *testing.T) {
	statsig, _, _ := SetupTest(t)
	defer statsig.Shutdown()

	const workers = 64
	const iterations = 2000

	noExposureConfigOpts := &statsig_go.DynamicConfigEvaluationOptions{DisableExposureLogging: true}
	noExposureExperimentOpts := &statsig_go.ExperimentEvaluationOptions{DisableExposureLogging: true}

	var wg sync.WaitGroup
	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()

			user, err := statsig_go.NewUserBuilderWithUserID(fmt.Sprintf("user-%d", id)).Build()
			if err != nil {
				t.Errorf("worker %d: failed to build user: %v", id, err)
				return
			}

			for i := 0; i < iterations; i++ {
				name := fmt.Sprintf("entity-%d-%d", id, i)

				config := statsig.GetDynamicConfigWithOptions(user, name, noExposureConfigOpts)
				if config.Name != name {
					t.Errorf("worker %d: GetDynamicConfig(%q) returned payload for %q", id, name, config.Name)
					return
				}

				experiment := statsig.GetExperimentWithOptions(user, name, noExposureExperimentOpts)
				if experiment.Name != name {
					t.Errorf("worker %d: GetExperiment(%q) returned payload for %q", id, name, experiment.Name)
					return
				}
			}
		}(w)
	}
	wg.Wait()
}
