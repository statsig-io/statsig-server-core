package main

import (
	"runtime"
	"testing"
	"time"

	statsig "github.com/statsig-io/private-statsig-server-core/statsig-go/src"
)

func BenchmarkOptionsMemoryLeak(b *testing.B) {

	optionsCount := 100

	for iter := 0; iter < b.N; iter++ {

		var memBefore, memAfter runtime.MemStats
		runtime.GC()
		runtime.ReadMemStats(&memBefore)

		optionsList := make([]*statsig.StatsigOptions, 0, optionsCount)
		for j := 0; j < optionsCount; j++ {
			options := statsig.NewStatsigOptionsBuilder().
				WithSpecsUrl("https://example.com/specs").
				WithLogEventUrl("https://example.com/log").
				WithEnvironment("dev").
				WithEventLoggingFlushIntervalMs(3000).
				WithEventLoggingMaxQueueSize(10000).
				WithSpecsSyncIntervalMs(1500).
				WithOutputLogLevel("WARN").
				Build()
			optionsList = append(optionsList, options)
		}
		optionsList = nil

		for j := 0; j < 5; j++ {
			runtime.GC()
			time.Sleep(100 * time.Millisecond)
		}

		runtime.ReadMemStats((&memAfter))
		memDiff := int64(memAfter.Alloc) - int64(memBefore.Alloc)
		b.ReportMetric(float64(memDiff), "mem_diff_bytes")

		if memDiff > 10000 {
			b.Errorf("Potential memory leak detected: allocated memory increased by %d bytes", memDiff)
		}
	}
}
