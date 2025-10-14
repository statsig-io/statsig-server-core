package statsig_go_core

import (
	"C"
	"fmt"
	"os"
	"runtime"
	"sync"

	"github.com/ebitengine/purego"
	"github.com/statsig-io/statsig-go-core/internal"
)

type StatsigFFI struct {
	lib uintptr

	// StatsigOptions
	statsig_options_create_from_data func(string) uint64
	statsig_options_release          func(uint64)

	// StatsigUser
	statsig_user_create_from_data func(string) uint64
	statsig_user_release          func(uint64)

	// Statsig Core
	statsig_create                           func(string, uint64) uint64
	statsig_release                          func(uint64)
	statsig_initialize                       func(uint64, func())
	statsig_initialize_with_details          func(uint64, func(string))
	statsig_initialize_with_details_blocking func(uint64) string
	statsig_initialize_blocking              func(uint64)
	statsig_shutdown                         func(uint64, func())
	statsig_shutdown_blocking                func(uint64)
	statsig_flush_events                     func(uint64, func())
	statsig_flush_events_blocking            func(uint64)
	statsig_get_current_values               func(uint64) string
	statsig_log_event                        func(uint64, uint64, string)
	statsig_identify                         func(uint64, uint64)
	statsig_get_client_init_response         func(uint64, uint64, string) string

	// Parameter Store
	statsig_get_parameter_store_with_options           func(uint64, string, string) string
	statsig_get_string_parameter_from_parameter_store  func(uint64, uint64, string, string, string, string) string
	statsig_get_bool_parameter_from_parameter_store    func(uint64, uint64, string, string, int, string) bool
	statsig_get_float64_parameter_from_parameter_store func(uint64, uint64, string, string, float64, string) float64
	statsig_get_int_parameter_from_parameter_store     func(uint64, uint64, string, string, int64, string) int64
	statsig_get_object_parameter_from_parameter_store  func(uint64, uint64, string, string, string, string) string
	statsig_get_array_parameter_from_parameter_store   func(uint64, uint64, string, string, string, string) string

	// Gates
	statsig_check_gate                 func(uint64, uint64, string, string) bool
	statsig_check_gate_performance     func(uint64, uint64, string, uint, string, uint) bool
	statsig_get_feature_gate           func(uint64, uint64, string, string) string
	statsig_manually_log_gate_exposure func(uint64, uint64, string)

	// Dynamic Configs
	statsig_get_dynamic_config                   func(uint64, uint64, string, string) string
	statsig_manually_log_dynamic_config_exposure func(uint64, uint64, string)

	// Experiments
	statsig_get_experiment                   func(uint64, uint64, string, string) string
	statsig_manually_log_experiment_exposure func(uint64, uint64, string)

	// Layers
	statsig_get_layer                             func(uint64, uint64, string, string) string
	statsig_log_layer_param_exposure              func(uint64, string, string)
	statsig_manually_log_layer_parameter_exposure func(uint64, uint64, string, string)

	// Prompts
	statsig_get_prompt func(uint64, uint64, string, string) string

	// Overrides
	statsig_override_gate                     func(uint64, string, bool, string)
	statsig_override_dynamic_config           func(uint64, string, string, string)
	statsig_override_experiment               func(uint64, string, string, string)
	statsig_override_experiment_by_group_name func(uint64, string, string, string)
	statsig_override_layer                    func(uint64, string, string, string)
	statsig_remove_gate_override              func(uint64, string, string)
	statsig_remove_dynamic_config_override    func(uint64, string, string)
	statsig_remove_experiment_override        func(uint64, string, string)
	statsig_remove_layer_override             func(uint64, string, string)
	statsig_remove_all_overrides              func(uint64)

	// Data Store
	data_store_create func(
		init_fn func(),
		shutdown_fn func(),
		get_fn func(key *C.char) *C.char,
		set_fn func(key *C.char, value *C.char, time *uint64),
		support_polling_updates_for_fn func(key *C.char) bool,
	) uint64
	data_store_release          func(uint64)
	__internal__test_data_store func(uint64, string, string) string

	// Metadata
	statsig_metadata_update_values func(string, string, string, string)

	// Utility
	free_string func(string)
}

var (
	instance *StatsigFFI
	once     sync.Once
)

func GetFFI() *StatsigFFI {
	once.Do(func() {
		lib, err := loadLibrary()
		if err != nil {
			panic(err)
		}

		instance = &StatsigFFI{
			lib: lib,
		}

		// StatsigOptions
		purego.RegisterLibFunc(&instance.statsig_options_create_from_data, lib, "statsig_options_create_from_data")
		purego.RegisterLibFunc(&instance.statsig_options_release, lib, "statsig_options_release")

		// StatsigUser
		purego.RegisterLibFunc(&instance.statsig_user_create_from_data, lib, "statsig_user_create_from_data")
		purego.RegisterLibFunc(&instance.statsig_user_release, lib, "statsig_user_release")

		// Statsig Core
		purego.RegisterLibFunc(&instance.statsig_create, lib, "statsig_create")
		purego.RegisterLibFunc(&instance.statsig_release, lib, "statsig_release")
		purego.RegisterLibFunc(&instance.statsig_initialize, lib, "statsig_initialize")
		purego.RegisterLibFunc(&instance.statsig_initialize_with_details, lib, "statsig_initialize_with_details")
		purego.RegisterLibFunc(&instance.statsig_initialize_with_details_blocking, lib, "statsig_initialize_with_details_blocking")
		purego.RegisterLibFunc(&instance.statsig_initialize_blocking, lib, "statsig_initialize_blocking")
		purego.RegisterLibFunc(&instance.statsig_shutdown, lib, "statsig_shutdown")
		purego.RegisterLibFunc(&instance.statsig_shutdown_blocking, lib, "statsig_shutdown_blocking")
		purego.RegisterLibFunc(&instance.statsig_flush_events, lib, "statsig_flush_events")
		purego.RegisterLibFunc(&instance.statsig_flush_events_blocking, lib, "statsig_flush_events_blocking")
		purego.RegisterLibFunc(&instance.statsig_get_current_values, lib, "statsig_get_current_values")
		purego.RegisterLibFunc(&instance.statsig_log_event, lib, "statsig_log_event")
		purego.RegisterLibFunc(&instance.statsig_identify, lib, "statsig_identify")
		purego.RegisterLibFunc(&instance.statsig_get_client_init_response, lib, "statsig_get_client_init_response")

		// Parameter Store
		purego.RegisterLibFunc(&instance.statsig_get_parameter_store_with_options, lib, "statsig_get_parameter_store_with_options")
		purego.RegisterLibFunc(&instance.statsig_get_string_parameter_from_parameter_store, lib, "statsig_get_string_parameter_from_parameter_store")
		purego.RegisterLibFunc(&instance.statsig_get_bool_parameter_from_parameter_store, lib, "statsig_get_bool_parameter_from_parameter_store")
		purego.RegisterLibFunc(&instance.statsig_get_float64_parameter_from_parameter_store, lib, "statsig_get_float64_parameter_from_parameter_store")
		purego.RegisterLibFunc(&instance.statsig_get_int_parameter_from_parameter_store, lib, "statsig_get_int_parameter_from_parameter_store")
		purego.RegisterLibFunc(&instance.statsig_get_object_parameter_from_parameter_store, lib, "statsig_get_object_parameter_from_parameter_store")
		purego.RegisterLibFunc(&instance.statsig_get_array_parameter_from_parameter_store, lib, "statsig_get_array_parameter_from_parameter_store")

		// Gates
		purego.RegisterLibFunc(&instance.statsig_check_gate, lib, "statsig_check_gate")
		purego.RegisterLibFunc(&instance.statsig_check_gate_performance, lib, "statsig_check_gate_performance")
		purego.RegisterLibFunc(&instance.statsig_get_feature_gate, lib, "statsig_get_feature_gate")
		purego.RegisterLibFunc(&instance.statsig_manually_log_gate_exposure, lib, "statsig_manually_log_gate_exposure")

		// Dynamic Configs
		purego.RegisterLibFunc(&instance.statsig_get_dynamic_config, lib, "statsig_get_dynamic_config")
		purego.RegisterLibFunc(&instance.statsig_manually_log_dynamic_config_exposure, lib, "statsig_manually_log_dynamic_config_exposure")

		// Experiments
		purego.RegisterLibFunc(&instance.statsig_get_experiment, lib, "statsig_get_experiment")
		purego.RegisterLibFunc(&instance.statsig_manually_log_experiment_exposure, lib, "statsig_manually_log_experiment_exposure")

		// Layers
		purego.RegisterLibFunc(&instance.statsig_get_layer, lib, "statsig_get_layer")
		purego.RegisterLibFunc(&instance.statsig_log_layer_param_exposure, lib, "statsig_log_layer_param_exposure")
		purego.RegisterLibFunc(&instance.statsig_manually_log_layer_parameter_exposure, lib, "statsig_manually_log_layer_parameter_exposure")

		// Prompts
		purego.RegisterLibFunc(&instance.statsig_get_prompt, lib, "statsig_get_prompt")

		// Overrides
		purego.RegisterLibFunc(&instance.statsig_override_gate, lib, "statsig_override_gate")
		purego.RegisterLibFunc(&instance.statsig_override_dynamic_config, lib, "statsig_override_dynamic_config")
		purego.RegisterLibFunc(&instance.statsig_override_experiment, lib, "statsig_override_experiment")
		purego.RegisterLibFunc(&instance.statsig_override_experiment_by_group_name, lib, "statsig_override_experiment_by_group_name")
		purego.RegisterLibFunc(&instance.statsig_override_layer, lib, "statsig_override_layer")
		purego.RegisterLibFunc(&instance.statsig_remove_gate_override, lib, "statsig_remove_gate_override")
		purego.RegisterLibFunc(&instance.statsig_remove_dynamic_config_override, lib, "statsig_remove_dynamic_config_override")
		purego.RegisterLibFunc(&instance.statsig_remove_experiment_override, lib, "statsig_remove_experiment_override")
		purego.RegisterLibFunc(&instance.statsig_remove_layer_override, lib, "statsig_remove_layer_override")
		purego.RegisterLibFunc(&instance.statsig_remove_all_overrides, lib, "statsig_remove_all_overrides")

		// Data Store
		purego.RegisterLibFunc(&instance.data_store_create, lib, "data_store_create")
		purego.RegisterLibFunc(&instance.data_store_release, lib, "data_store_release")
		purego.RegisterLibFunc(&instance.__internal__test_data_store, lib, "__internal__test_data_store")

		// Metadata
		purego.RegisterLibFunc(&instance.statsig_metadata_update_values, lib, "statsig_metadata_update_values")

		// Utility
		purego.RegisterLibFunc(&instance.free_string, lib, "free_string")

		instance.updateStatsigMetadata()
	})

	return instance
}

func (ffi *StatsigFFI) updateStatsigMetadata() {
	ffi.statsig_metadata_update_values(
		"statsig-server-core-go",
		runtime.GOOS,
		runtime.GOARCH,
		runtime.Version(),
	)
}

func loadLibrary() (uintptr, error) {
	flags := purego.RTLD_NOW | purego.RTLD_GLOBAL

	if p := os.Getenv("STATSIG_LIB_PATH"); p != "" {
		h, err := purego.Dlopen(p, flags)
		if err != nil {
			return 0, fmt.Errorf("STATSIG_LIB_PATH is set but could not be loaded: %w", err)
		}
		return h, nil
	}

	path, err := writeBinaryToTempFile()
	if err != nil {
		return 0, fmt.Errorf("failed to write binary to temp file: %w", err)
	}
	h, err := purego.Dlopen(path, flags)
	if err != nil {
		return 0, fmt.Errorf("failed to open library: %w", err)
	}
	_ = os.Remove(path)
	return h, nil
}

func writeBinaryToTempFile() (string, error) {
	f, err := os.CreateTemp("", "statsiglib")
	if err != nil {
		return "", fmt.Errorf("failed to create temp file: %w", err)
	}
	defer f.Close()

	if _, err = f.Write(internal.GetLibData()); err != nil {
		return "", fmt.Errorf("failed to write binary to temp file: %w", err)
	}
	return f.Name(), nil
}
