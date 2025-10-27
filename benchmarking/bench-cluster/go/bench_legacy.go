package main

import (
	"debug/buildinfo"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"math/rand"
	"os"
	"sort"
	"time"

	statsig "github.com/statsig-io/go-sdk"
)

const (
	SCAPI_URL  = "http://scrapi:8000"
	ITER_LITE  = 1000
	ITER_HEAVY = 10000
)

type BenchmarkResult struct {
	BenchmarkName string  `json:"benchmarkName"`
	P99           float64 `json:"p99"`
	Max           float64 `json:"max"`
	Min           float64 `json:"min"`
	Median        float64 `json:"median"`
	Avg           float64 `json:"avg"`
	SpecName      string  `json:"specName"`
	SDKType       string  `json:"sdkType"`
	SDKVersion    string  `json:"sdkVersion"`
}

type SpecNames struct {
	FeatureGates   []string `json:"feature_gates"`
	DynamicConfigs []string `json:"dynamic_configs"`
	Experiments    []string `json:"experiments"`
	Layers         []string `json:"layers"`
}

func BenchLegacy() {
	sdkVersion := getSdkVersion()

	fmt.Printf("Statsig Go Legacy (v%s)\n", sdkVersion)
	fmt.Println("--------------------------------")

	specNames := loadSpecNames()

	options := statsig.Options{
		API: fmt.Sprintf("%s/v1", SCAPI_URL),
	}

	var results []BenchmarkResult

	statsig.InitializeWithOptions("secret-GO_LEGACY", &options)

	globalUser := statsig.User{
		UserID: "global_user",
	}

	// Benchmark feature gates
	for _, gateName := range specNames.FeatureGates {
		benchmark(&results, "check_gate", gateName, ITER_HEAVY, "go-sdk", func() {
			user := createUser()
			statsig.CheckGate(user, gateName)
		})

		benchmark(&results, "check_gate_global_user", gateName, ITER_HEAVY, "go-sdk", func() {
			statsig.CheckGate(globalUser, gateName)
		})

		benchmark(&results, "get_feature_gate", gateName, ITER_HEAVY, "go-sdk", func() {
			user := createUser()
			statsig.GetGate(user, gateName)
		})

		benchmark(&results, "get_feature_gate_global_user", gateName, ITER_HEAVY, "go-sdk", func() {
			statsig.GetGate(globalUser, gateName)
		})
	}

	// Benchmark dynamic configs
	for _, configName := range specNames.DynamicConfigs {
		benchmark(&results, "get_dynamic_config", configName, ITER_HEAVY, "go-sdk", func() {
			user := createUser()
			statsig.GetConfig(user, configName)
		})

		benchmark(&results, "get_dynamic_config_global_user", configName, ITER_HEAVY, "go-sdk", func() {
			statsig.GetConfig(globalUser, configName)
		})
	}

	// Benchmark experiments
	for _, experimentName := range specNames.Experiments {
		benchmark(&results, "get_experiment", experimentName, ITER_HEAVY, "go-sdk", func() {
			user := createUser()
			statsig.GetExperiment(user, experimentName)
		})

		benchmark(&results, "get_experiment_global_user", experimentName, ITER_HEAVY, "go-sdk", func() {
			statsig.GetExperiment(globalUser, experimentName)
		})
	}

	// Benchmark layers
	for _, layerName := range specNames.Layers {
		benchmark(&results, "get_layer", layerName, ITER_HEAVY, "go-sdk", func() {
			user := createUser()
			statsig.GetLayer(user, layerName)
		})

		benchmark(&results, "get_layer_global_user", layerName, ITER_HEAVY, "go-sdk", func() {
			statsig.GetLayer(globalUser, layerName)
		})
	}

	statsig.Shutdown()

	writeResults(&results, "go-sdk", sdkVersion)
}

func loadSpecNames() SpecNames {
	path := "/shared-volume/spec_names.json"
	for i := 1; i <= 10; i++ {
		if _, err := os.Stat(path); err == nil {
			break
		}
		fmt.Printf("(%d/10) Waiting for spec_names.json to be created...\n", i)
		time.Sleep(time.Second)
	}

	data, err := ioutil.ReadFile(path)
	if err != nil {
		panic(fmt.Sprintf("Failed to read spec_names.json: %v", err))
	}

	var specNames SpecNames
	if err := json.Unmarshal(data, &specNames); err != nil {
		panic(fmt.Sprintf("Failed to parse spec_names.json: %v", err))
	}

	return specNames
}

func getSdkVersion() string {
	info, err := buildinfo.ReadFile(os.Args[0])
	if err != nil {
		panic(fmt.Sprintf("Failed to read build info: %v", err))
	}

	for _, dep := range info.Deps {
		if dep.Path == "github.com/statsig-io/go-sdk" {
			return dep.Version
		}
	}

	panic("Statsig SDK dependency not found")
}

func createUser() statsig.User {
	rnd := rand.Intn(1000000)
	user := statsig.User{
		UserID:            fmt.Sprintf("user_%d", rnd),
		Email:             "user@example.com",
		IpAddress:         "127.0.0.1",
		Locale:            "en-US",
		AppVersion:        "1.0.0",
		Country:           "US",
		UserAgent:         "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
		Custom:            map[string]interface{}{"isAdmin": false},
		PrivateAttributes: map[string]interface{}{"isPaid": "nah"},
	}

	return user
}

func benchmark(results *[]BenchmarkResult, benchmarkName, specName string, iterations int, sdkType string, fn func()) {
	durations := make([]float64, iterations)

	for i := 0; i < iterations; i++ {
		start := time.Now()
		fn()
		duration := time.Since(start)
		durations[i] = float64(duration.Nanoseconds()) / 1e6 // Convert to milliseconds
	}

	sort.Float64s(durations)

	sdkVersion := getSdkVersion()
	result := BenchmarkResult{
		BenchmarkName: benchmarkName,
		SpecName:      specName,
		SDKType:       sdkType,
		SDKVersion:    sdkVersion,
		P99:           durations[len(durations)*99/100],
		Max:           durations[len(durations)-1],
		Min:           durations[0],
		Median:        durations[len(durations)/2],
		Avg:           calculateAverage(durations),
	}

	fmt.Printf("%-30s p99(ms): %.4f max(ms): %.4f %s\n", benchmarkName, result.P99, result.Max, specName)

	*results = append(*results, result)
}

func calculateAverage(values []float64) float64 {
	sum := 0.0
	for _, v := range values {
		sum += v
	}
	return sum / float64(len(values))
}
