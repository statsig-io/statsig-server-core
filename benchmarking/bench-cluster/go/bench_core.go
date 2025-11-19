package main

import (
	"debug/buildinfo"
	"fmt"
	"math/rand"
	"os"

	statsig "github.com/statsig-io/statsig-go-core"
	statsig_go_core "github.com/statsig-io/statsig-go-core"
)

const SDK_TYPE = "statsig-server-core-go"

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

	// ------------------------------------------------------------------------ [ Benchmark Feature Gates ]

	for _, gateName := range specNames.FeatureGates {
		benchmark(&results, "check_gate", gateName, ITER_HEAVY, SDK_TYPE, func() {
			user := createCoreUser()
			s.CheckGate(user, gateName)
		})

		benchmark(&results, "check_gate_global_user", gateName, ITER_HEAVY, SDK_TYPE, func() {
			s.CheckGate(globalUser, gateName)
		})

		benchmark(&results, "get_feature_gate", gateName, ITER_HEAVY, SDK_TYPE, func() {
			user := createCoreUser()
			s.GetFeatureGate(user, gateName)
		})

		benchmark(&results, "get_feature_gate_global_user", gateName, ITER_HEAVY, SDK_TYPE, func() {
			s.GetFeatureGate(globalUser, gateName)
		})
	}

	// ------------------------------------------------------------------------ [ Benchmark Dynamic Configs ]

	for _, configName := range specNames.DynamicConfigs {
		benchmark(&results, "get_dynamic_config", configName, ITER_HEAVY, SDK_TYPE, func() {
			user := createCoreUser()
			s.GetDynamicConfig(user, configName)
		})

		benchmark(&results, "get_dynamic_config_global_user", configName, ITER_HEAVY, SDK_TYPE, func() {
			s.GetDynamicConfig(globalUser, configName)
		})
	}

	config := s.GetDynamicConfig(globalUser, "operating_system_config")
	benchmarkWithArgs(config, &results, "get_dynamic_config_params", "string", ITER_HEAVY, SDK_TYPE, func(config statsig_go_core.DynamicConfig) {
		v := config.GetString("str", "err")
		if v == "err" {
			panic("string value is err")
		}
	})

	benchmarkWithArgs(config, &results, "get_dynamic_config_params", "number", ITER_HEAVY, SDK_TYPE, func(config statsig_go_core.DynamicConfig) {
		v := config.GetNumber("num", 0)
		if v == 0 {
			panic("number value is 0")
		}
	})

	benchmarkWithArgs(config, &results, "get_dynamic_config_params", "object", ITER_HEAVY, SDK_TYPE, func(config statsig_go_core.DynamicConfig) {
		v := config.GetMap("obj", map[string]any{})
		if len(v) == 0 {
			panic("object value is empty")
		}
	})

	benchmarkWithArgs(config, &results, "get_dynamic_config_params", "array", ITER_HEAVY, SDK_TYPE, func(config statsig_go_core.DynamicConfig) {
		v := config.GetSlice("arr", []any{})
		if len(v) == 0 {
			panic("array value is empty")
		}
	})

	// ------------------------------------------------------------------------ [ Benchmark Experiments ]

	for _, experimentName := range specNames.Experiments {
		benchmark(&results, "get_experiment", experimentName, ITER_HEAVY, SDK_TYPE, func() {
			user := createCoreUser()
			s.GetExperiment(user, experimentName)
		})

		benchmark(&results, "get_experiment_global_user", experimentName, ITER_HEAVY, SDK_TYPE, func() {
			s.GetExperiment(globalUser, experimentName)
		})
	}

	experiment := s.GetExperiment(globalUser, "experiment_with_many_params")
	benchmarkWithArgs(experiment, &results, "get_experiment_params", "string", ITER_HEAVY, SDK_TYPE, func(experiment statsig_go_core.Experiment) {
		v := experiment.GetString("a_string", "err")
		if v == "err" {
			panic("string value is err")
		}
	})

	benchmarkWithArgs(experiment, &results, "get_experiment_params", "object", ITER_HEAVY, SDK_TYPE, func(experiment statsig_go_core.Experiment) {
		v := experiment.GetMap("an_object", map[string]any{})
		if len(v) == 0 {
			panic("object value is empty")
		}
	})

	benchmarkWithArgs(experiment, &results, "get_experiment_params", "array", ITER_HEAVY, SDK_TYPE, func(experiment statsig_go_core.Experiment) {
		v := experiment.GetSlice("an_array", []any{})
		if len(v) == 0 {
			panic("array value is empty")
		}
	})

	// ------------------------------------------------------------------------ [ Benchmark Layers ]

	for _, layerName := range specNames.Layers {
		benchmark(&results, "get_layer", layerName, ITER_HEAVY, SDK_TYPE, func() {
			user := createCoreUser()
			s.GetLayer(user, layerName)
		})

		benchmark(&results, "get_layer_global_user", layerName, ITER_HEAVY, SDK_TYPE, func() {
			s.GetLayer(globalUser, layerName)
		})
	}

	layer := s.GetLayer(globalUser, "layer_with_many_params")
	benchmarkWithArgs(layer, &results, "get_layer_params", "string", ITER_HEAVY, SDK_TYPE, func(layer statsig_go_core.Layer) {
		v := layer.GetString("a_string", "err")
		if v == "err" {
			panic("string value is err")
		}
	})

	benchmarkWithArgs(layer, &results, "get_layer_params", "object", ITER_HEAVY, SDK_TYPE, func(layer statsig_go_core.Layer) {
		v := layer.GetMap("an_object", map[string]any{})
		if len(v) == 0 {
			panic("object value is empty")
		}
	})

	benchmarkWithArgs(layer, &results, "get_layer_params", "array", ITER_HEAVY, SDK_TYPE, func(layer statsig_go_core.Layer) {
		v := layer.GetSlice("an_array", []any{})
		if len(v) == 0 {
			panic("array value is empty")
		}
	})

	// ------------------------------------------------------------------------ [ Benchmark Client Initialize Response ]

	benchmark(&results, "get_client_initialize_response", "n/a", ITER_LITE, SDK_TYPE, func() {
		user := createCoreUser()
		s.GetClientInitResponse(user)
	})
	benchmark(&results, "get_client_initialize_response_global_user", "n/a", ITER_LITE, SDK_TYPE, func() {
		s.GetClientInitResponse(globalUser)
	})

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
