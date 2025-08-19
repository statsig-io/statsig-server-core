package tests

import (
	"strings"
	"sync"
	"testing"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

func TestSingletonBasic(t *testing.T) {
	_ = statsig.NewShared("secret-key", statsig.StatsigOptions{})
	global := statsig.Shared()

	if global == nil {
		t.Errorf("Error: global statsig instance expected to be not nil")
	}
	statsig.RemoveSharedInstance()
}

func TestSingletonDoubleCreation(t *testing.T) {
	var global *statsig.Statsig
	var incorrectGlobal *statsig.Statsig
	firstInstance := getOutputStream((func() {
		global = statsig.NewShared("secret-key", statsig.StatsigOptions{})
	}))

	if !(strings.Contains(firstInstance, "Creating a new shared instance")) {
		t.Errorf("Error, did not create a new shared instance, instead got %v", firstInstance)
	}

	secondInstance := getOutputStream(func() {
		incorrectGlobal = statsig.NewShared("secret-key", statsig.StatsigOptions{})
	})

	if !(strings.Contains(secondInstance, "Shared instance has been created, call removeShared() if you want to create another one.")) {
		t.Errorf("Error, did not err, instead got %v", secondInstance)
	}

	setShared := statsig.Shared()

	if setShared != global {
		t.Errorf("Error, getting shared instance did not return the expected Statsig object")
	}

	if setShared == incorrectGlobal {
		t.Errorf("Error, getting error statsig instance returned the wrong Statsig object")
	}

	statsig.RemoveSharedInstance()
}

func TestSingletonRemove(t *testing.T) {
	globalStatsig := statsig.NewShared("secret-key", statsig.StatsigOptions{})

	if globalStatsig == nil {
		t.Errorf("Error, shared statsig instance not created")
	}

	statsig.RemoveSharedInstance()

	strOutput := getOutputStream(func() {
		statsig.Shared()
	})

	if !(strings.Contains(strOutput, "[Statsig] No shared instance has been created yet. Call newShared() before using it. Returning an invalid instance")) {
		t.Errorf("Error, shared instance detected when shared instance was removed")
	}

	statsig.RemoveSharedInstance()
}

func TestSingletonConcurrency(t *testing.T) {
	const goroutines = 10
	results := make([]*statsig.Statsig, goroutines)
	var wg sync.WaitGroup
	wg.Add(goroutines)

	getOutput := getOutputStream(func() {
		for i := 0; i < goroutines; i++ {
			go func(idx int) {
				defer wg.Done()
				results[idx] = statsig.NewShared("secret-key", statsig.StatsigOptions{})
			}(i)
		}

		wg.Wait()
	})

	if strings.Count(getOutput, "Shared instance has been created, call removeShared() if you want to create another one.") != 9 {
		t.Errorf("Expected error message 10 times")
	}

	if strings.Count(getOutput, "Creating a new shared instance") != 1 {
		t.Errorf("Expected shared instance to only be created once")
	}
}
