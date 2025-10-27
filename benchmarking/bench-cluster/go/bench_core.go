package main

import (
	"debug/buildinfo"
	"fmt"
	"math/rand"
	"os"

	statsig "github.com/statsig-io/statsig-go-core"
)

func BenchCore() {
	sdkVersion := getCoreSdkVersion()

	fmt.Printf("Statsig Go Core (%s)\n", sdkVersion)
	fmt.Println("--------------------------------")

	specNames := loadSpecNames()

	options, err := statsig.NewOptionsBuilder().
		WithSpecsUrl(fmt.Sprintf("%s/v2/download_config_specs", SCAPI_URL)).
		WithLogEventUrl(fmt.Sprintf("%s/v1/log_event", SCAPI_URL)).
		Build()

	if err != nil {
		panic(fmt.Sprintf("Failed to create options: %v", err))
	}

	var results []BenchmarkResult

	s, _ := statsig.NewStatsigWithOptions("secret-GO_CORE", options)
	s.Initialize()

	globalUser, err := statsig.NewUserBuilderWithUserID("global_user").Build()

	if err != nil {
		panic(fmt.Sprintf("Failed to create global user: %v", err))
	}

	// Benchmark feature gates
	for _, gateName := range specNames.FeatureGates {
		benchmark(&results, "check_gate", gateName, ITER_HEAVY, "statsig-server-core-go", func() {
			user := createCoreUser()
			s.CheckGate(user, gateName)
		})

		benchmark(&results, "check_gate_global_user", gateName, ITER_HEAVY, "statsig-server-core-go", func() {
			s.CheckGate(globalUser, gateName)
		})

		benchmark(&results, "get_feature_gate", gateName, ITER_HEAVY, "statsig-server-core-go", func() {
			user := createCoreUser()
			s.GetFeatureGate(user, gateName)
		})

		benchmark(&results, "get_feature_gate_global_user", gateName, ITER_HEAVY, "statsig-server-core-go", func() {
			s.GetFeatureGate(globalUser, gateName)
		})
	}

	// Benchmark dynamic configs
	for _, configName := range specNames.DynamicConfigs {
		benchmark(&results, "get_dynamic_config", configName, ITER_HEAVY, "statsig-server-core-go", func() {
			user := createCoreUser()
			s.GetDynamicConfig(user, configName)
		})

		benchmark(&results, "get_dynamic_config_global_user", configName, ITER_HEAVY, "statsig-server-core-go", func() {
			s.GetDynamicConfig(globalUser, configName)
		})
	}

	// Benchmark experiments
	for _, experimentName := range specNames.Experiments {
		benchmark(&results, "get_experiment", experimentName, ITER_HEAVY, "statsig-server-core-go", func() {
			user := createCoreUser()
			s.GetExperiment(user, experimentName)
		})

		benchmark(&results, "get_experiment_global_user", experimentName, ITER_HEAVY, "statsig-server-core-go", func() {
			s.GetExperiment(globalUser, experimentName)
		})
	}

	// Benchmark layers
	for _, layerName := range specNames.Layers {
		benchmark(&results, "get_layer", layerName, ITER_HEAVY, "statsig-server-core-go", func() {
			user := createCoreUser()
			s.GetLayer(user, layerName)
		})

		benchmark(&results, "get_layer_global_user", layerName, ITER_HEAVY, "statsig-server-core-go", func() {
			s.GetLayer(globalUser, layerName)
		})
	}

	s.Shutdown()

	writeResults(&results, "statsig-server-core-go", sdkVersion)
}

func getCoreSdkVersion() string {
	info, err := buildinfo.ReadFile(os.Args[0])
	if err != nil {
		panic(fmt.Sprintf("Failed to read build info: %v", err))
	}

	for _, dep := range info.Deps {
		if dep.Path == "github.com/statsig-io/statsig-go-core" {
			return dep.Version
		}
	}

	panic("Statsig SDK dependency not found")
}

func createCoreUser() *statsig.StatsigUser {
	rnd := rand.Intn(1000000)
	user, err := statsig.NewUserBuilderWithUserID(fmt.Sprintf("user_%d", rnd)).
		WithEmail("user@example.com").
		WithIpAddress("127.0.0.1").
		WithLocale("en-US").
		WithAppVersion("1.0.0").
		WithCountry("US").
		WithUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36").
		WithCustom(map[string]interface{}{"isAdmin": false}).
		WithPrivateAttributes(map[string]interface{}{"isPaid": "nah"}).
		Build()

	if err != nil {
		panic(fmt.Sprintf("Failed to create user: %v", err))
	}

	return user
}
