package main

import (
	"runtime"
	"testing"
	"time"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func BenchmarkUserMemoryLeak(b *testing.B) {

	var usersCount = 100

	for iter := 0; iter < b.N; iter++ {
		var memStatsBefore, memStatsAfter runtime.MemStats

		runtime.GC()
		runtime.ReadMemStats(&memStatsBefore)

		users := make([]*statsig.StatsigUser, 0, usersCount)
		for i := 0; i < usersCount; i++ {
			user := statsig.NewStatsigUserBuilder().
				WithUserID("test-user0").
				WithEmail("test@test.com").
				WithIpAddress("127.0.0.1").
				WithUserAgent("test-user-agent").
				WithCountry("US").
				WithLocale("en-US").
				WithAppVersion("1.0.0").
				WithCustom(map[string]interface{}{
					"feature_enabled":  true,
					"experiment_group": "beta_group_3",
				}).
				WithPrivateAttributes(map[string]interface{}{
					"app_build_number": 204,
					"nested": map[string]interface{}{
						"sub_key": "nested_value",
					},
				}).
				WithStatsigEnvironment(map[string]string{
					"appVersion": "2.3.1",
				}).
				WithCustomIds(map[string]string{
					"stable_id": "user-stable-9876",
				}).
				Build()
			users = append(users, user)

		}

		// Drop reference to users to allow GC
		users = nil

		for i := 0; i < 5; i++ {
			runtime.GC()
			time.Sleep(50 * time.Millisecond)
		}

		runtime.ReadMemStats(&memStatsAfter)

		memDiff := int64(memStatsAfter.Alloc) - int64(memStatsBefore.Alloc)

		// Report memory difference as a benchmark metric
		b.ReportMetric(float64(memDiff), "mem_diff_bytes")

		if memDiff > 10000 {
			b.Errorf("Potential memory leak detected: allocated memory increased by %d bytes", memDiff)
		}
	}
}
