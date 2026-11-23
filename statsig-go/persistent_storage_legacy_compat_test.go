package statsig_go_core_test

import (
	statsig "github.com/statsig-io/statsig-go-core"
	"testing"
)

// External-package compile fixtures deliberately use legacy positional literals.
// Adding a field, changing a callback, or widening the legacy map breaks these.
var legacyRecord = statsig.StickyValues{true, map[string]string{"s": "v"}, "rule", nil, []statsig.SecondaryExposure{}, nil, nil, nil, 1, nil}
var legacyKeyed = statsig.StickyValues{JSONValue: map[string]string{"s": "v"}}
var legacyMap = statsig.UserPersistedValues{"exp": legacyRecord}
var legacyFunctions = statsig.PersistentStorageFunctions{
	func(string) *statsig.UserPersistedValues { return &legacyMap },
	func(key, name string, v statsig.StickyValues) {
		var s string = v.JSONValue["s"]
		acceptLegacyStrings(v.JSONValue)
		_ = s
	},
	func(string, string) {},
}
var _ func(statsig.PersistentStorageFunctions) *statsig.PersistentStorage = statsig.NewPersistentStorage
var _ = statsig.ExperimentEvaluationOptions{true, &legacyMap, true, true}
var _ = statsig.LayerEvaluationOptions{true, &legacyMap, true, true}
var _ func(*statsig.Statsig, *statsig.StatsigUser, string, *statsig.ExperimentEvaluationOptions) statsig.Experiment = (*statsig.Statsig).GetExperimentWithOptions
var _ func(*statsig.Statsig, *statsig.StatsigUser, string, *statsig.LayerEvaluationOptions) statsig.Layer = (*statsig.Statsig).GetLayerWithOptions

func acceptLegacyStrings(map[string]string) {}
func TestPersistentV2LegacyConsumerCompiles(t *testing.T) {
	legacyFunctions.Save("k", "exp", legacyKeyed)
}
