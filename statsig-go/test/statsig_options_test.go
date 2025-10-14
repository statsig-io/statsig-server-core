package test

import (
	"fmt"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestStatsigOptionsBuilder(t *testing.T) {
	fmt.Println("TestStatsigOptionsBuilder")
	_, err := statsig_go.NewOptionsBuilder().WithSpecsUrl("http://localhost:3000/specs").Build()

	if err != nil {
		t.Errorf("error creating StatsigOptions: %v", err)
	}
}

func TestStatsigOptionsBuilderArgs(t *testing.T) {
	builder := statsig_go.StatsigOptionsBuilder{
		SpecsUrl:                    ptr("http://localhost:3000/specs"),
		LogEventUrl:                 ptr("http://localhost:3000/events"),
		Environment:                 ptr("production"),
		SpecsSyncIntervalMs:         ptr(int32(1000)),
		EventLoggingFlushIntervalMs: ptr(int32(2000)),
		EventLoggingMaxQueueSize:    ptr(int32(5000)),
		WaitForCountryLookupInit:    ptr(true),
		WaitForUserAgentInit:        ptr(true),
	}

	_, err := builder.Build()

	if err != nil {
		t.Errorf("error creating StatsigOptions: %v", err)
	}
}

func ptr[T any](v T) *T {
	return &v
}
