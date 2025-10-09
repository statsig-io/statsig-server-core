package statsig_go_core

import (
	"fmt"
	"os"
	"runtime"

	"github.com/ebitengine/purego"
	"github.com/statsig-io/statsig-go-core/internal"
)

// string <=> char*
// bool <=> _Bool
// uintptr <=> uintptr_t
// uint <=> uint32_t or uint64_t
// uint8 <=> uint8_t
// uint16 <=> uint16_t
// uint32 <=> uint32_t
// uint64 <=> uint64_t
// int <=> int32_t or int64_t
// int8 <=> int8_t
// int16 <=> int16_t
// int32 <=> int32_t
// int64 <=> int64_t
// float32 <=> float
// float64 <=> double
// struct <=> struct (WIP - darwin only)
// func <=> C function
// unsafe.Pointer, *T <=> void*
// []T => void*

type StatsigFFI struct {
	lib uintptr

	// StatsigOptions
	statsig_options_create_from_data func(string) uint64
	statsig_options_release          func(uint64)

	// StatsigUser
	statsig_user_create  func(string, string, *string, *string, *string, *string, *string, *string, *string, *string) uint64
	statsig_user_release func(uint64)

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

	// Metadata
	statsig_metadata_update_values func(string, string, string, string)

	// Utility
	free_string func(string)
}

var instance *StatsigFFI

func GetFFI() *StatsigFFI {
	if instance != nil {
		return instance
	}

	lib, err := loadLibrary()
	if err != nil {
		panic(err)
	}

	// StatsigOptions
	var statsig_options_create_from_data func(string) uint64
	purego.RegisterLibFunc(&statsig_options_create_from_data, lib, "statsig_options_create_from_data")

	var statsig_options_release func(uint64)
	purego.RegisterLibFunc(&statsig_options_release, lib, "statsig_options_release")

	// StatsigUser
	var statsig_user_create func(
		string, // user_id
		string, // custom_ids_json
		*string, // email
		*string, // ip
		*string, // user_agent
		*string, // country
		*string, // locale
		*string, // app_version
		*string, // custom_json
		*string, // private_attributes_json
	) uint64
	purego.RegisterLibFunc(&statsig_user_create, lib, "statsig_user_create")

	var statsig_user_release func(uint64)
	purego.RegisterLibFunc(&statsig_user_release, lib, "statsig_user_release")

	// Statsig Core
	var statsig_create func(string, uint64) uint64
	purego.RegisterLibFunc(&statsig_create, lib, "statsig_create")

	var statsig_release func(uint64)
	purego.RegisterLibFunc(&statsig_release, lib, "statsig_release")

	var statsig_initialize func(uint64, func())
	purego.RegisterLibFunc(&statsig_initialize, lib, "statsig_initialize")

	var statsig_initialize_with_details func(uint64, func(string))
	purego.RegisterLibFunc(&statsig_initialize_with_details, lib, "statsig_initialize_with_details")

	var statsig_initialize_with_details_blocking func(uint64) string
	purego.RegisterLibFunc(&statsig_initialize_with_details_blocking, lib, "statsig_initialize_with_details_blocking")

	var statsig_initialize_blocking func(uint64)
	purego.RegisterLibFunc(&statsig_initialize_blocking, lib, "statsig_initialize_blocking")

	var statsig_shutdown func(uint64, func())
	purego.RegisterLibFunc(&statsig_shutdown, lib, "statsig_shutdown")

	var statsig_shutdown_blocking func(uint64)
	purego.RegisterLibFunc(&statsig_shutdown_blocking, lib, "statsig_shutdown_blocking")

	var statsig_flush_events func(uint64, func())
	purego.RegisterLibFunc(&statsig_flush_events, lib, "statsig_flush_events")

	var statsig_flush_events_blocking func(uint64)
	purego.RegisterLibFunc(&statsig_flush_events_blocking, lib, "statsig_flush_events_blocking")

	var statsig_get_current_values func(uint64) string
	purego.RegisterLibFunc(&statsig_get_current_values, lib, "statsig_get_current_values")

	var statsig_log_event func(uint64, uint64, string)
	purego.RegisterLibFunc(&statsig_log_event, lib, "statsig_log_event")

	var statsig_identify func(uint64, uint64)
	purego.RegisterLibFunc(&statsig_identify, lib, "statsig_identify")

	var statsig_get_client_init_response func(uint64, uint64, string) string
	purego.RegisterLibFunc(&statsig_get_client_init_response, lib, "statsig_get_client_init_response")

	// Parameter Store
	var statsig_get_parameter_store_with_options func(uint64, string, string) string
	purego.RegisterLibFunc(&statsig_get_parameter_store_with_options, lib, "statsig_get_parameter_store_with_options")

	var statsig_get_string_parameter_from_parameter_store func(uint64, uint64, string, string, string, string) string
	purego.RegisterLibFunc(&statsig_get_string_parameter_from_parameter_store, lib, "statsig_get_string_parameter_from_parameter_store")

	var statsig_get_bool_parameter_from_parameter_store func(uint64, uint64, string, string, int, string) bool
	purego.RegisterLibFunc(&statsig_get_bool_parameter_from_parameter_store, lib, "statsig_get_bool_parameter_from_parameter_store")

	var statsig_get_float64_parameter_from_parameter_store func(uint64, uint64, string, string, float64, string) float64
	purego.RegisterLibFunc(&statsig_get_float64_parameter_from_parameter_store, lib, "statsig_get_float64_parameter_from_parameter_store")

	var statsig_get_int_parameter_from_parameter_store func(uint64, uint64, string, string, int64, string) int64
	purego.RegisterLibFunc(&statsig_get_int_parameter_from_parameter_store, lib, "statsig_get_int_parameter_from_parameter_store")

	var statsig_get_object_parameter_from_parameter_store func(uint64, uint64, string, string, string, string) string
	purego.RegisterLibFunc(&statsig_get_object_parameter_from_parameter_store, lib, "statsig_get_object_parameter_from_parameter_store")

	var statsig_get_array_parameter_from_parameter_store func(uint64, uint64, string, string, string, string) string
	purego.RegisterLibFunc(&statsig_get_array_parameter_from_parameter_store, lib, "statsig_get_array_parameter_from_parameter_store")

	// Gates
	var statsig_check_gate func(uint64, uint64, string, string) bool
	purego.RegisterLibFunc(&statsig_check_gate, lib, "statsig_check_gate")

	var statsig_check_gate_performance func(uint64, uint64, string, uint, string, uint) bool
	purego.RegisterLibFunc(&statsig_check_gate_performance, lib, "statsig_check_gate_performance")

	var statsig_get_feature_gate func(uint64, uint64, string, string) string
	purego.RegisterLibFunc(&statsig_get_feature_gate, lib, "statsig_get_feature_gate")

	var statsig_manually_log_gate_exposure func(uint64, uint64, string)
	purego.RegisterLibFunc(&statsig_manually_log_gate_exposure, lib, "statsig_manually_log_gate_exposure")

	// Dynamic Configs
	var statsig_get_dynamic_config func(uint64, uint64, string, string) string
	purego.RegisterLibFunc(&statsig_get_dynamic_config, lib, "statsig_get_dynamic_config")

	var statsig_manually_log_dynamic_config_exposure func(uint64, uint64, string)
	purego.RegisterLibFunc(&statsig_manually_log_dynamic_config_exposure, lib, "statsig_manually_log_dynamic_config_exposure")

	// Experiments
	var statsig_get_experiment func(uint64, uint64, string, string) string
	purego.RegisterLibFunc(&statsig_get_experiment, lib, "statsig_get_experiment")

	var statsig_manually_log_experiment_exposure func(uint64, uint64, string)
	purego.RegisterLibFunc(&statsig_manually_log_experiment_exposure, lib, "statsig_manually_log_experiment_exposure")

	// Layers
	var statsig_get_layer func(uint64, uint64, string, string) string
	purego.RegisterLibFunc(&statsig_get_layer, lib, "statsig_get_layer")

	var statsig_log_layer_param_exposure func(uint64, string, string)
	purego.RegisterLibFunc(&statsig_log_layer_param_exposure, lib, "statsig_log_layer_param_exposure")

	var statsig_manually_log_layer_parameter_exposure func(uint64, uint64, string, string)
	purego.RegisterLibFunc(&statsig_manually_log_layer_parameter_exposure, lib, "statsig_manually_log_layer_parameter_exposure")

	// Prompts
	var statsig_get_prompt func(uint64, uint64, string, string) string
	purego.RegisterLibFunc(&statsig_get_prompt, lib, "statsig_get_prompt")

	// Overrides
	var statsig_override_gate func(uint64, string, bool, string)
	purego.RegisterLibFunc(&statsig_override_gate, lib, "statsig_override_gate")

	var statsig_override_dynamic_config func(uint64, string, string, string)
	purego.RegisterLibFunc(&statsig_override_dynamic_config, lib, "statsig_override_dynamic_config")

	var statsig_override_experiment func(uint64, string, string, string)
	purego.RegisterLibFunc(&statsig_override_experiment, lib, "statsig_override_experiment")

	var statsig_override_experiment_by_group_name func(uint64, string, string, string)
	purego.RegisterLibFunc(&statsig_override_experiment_by_group_name, lib, "statsig_override_experiment_by_group_name")

	var statsig_override_layer func(uint64, string, string, string)
	purego.RegisterLibFunc(&statsig_override_layer, lib, "statsig_override_layer")

	var statsig_remove_gate_override func(uint64, string, string)
	purego.RegisterLibFunc(&statsig_remove_gate_override, lib, "statsig_remove_gate_override")

	var statsig_remove_dynamic_config_override func(uint64, string, string)
	purego.RegisterLibFunc(&statsig_remove_dynamic_config_override, lib, "statsig_remove_dynamic_config_override")

	var statsig_remove_experiment_override func(uint64, string, string)
	purego.RegisterLibFunc(&statsig_remove_experiment_override, lib, "statsig_remove_experiment_override")

	var statsig_remove_layer_override func(uint64, string, string)
	purego.RegisterLibFunc(&statsig_remove_layer_override, lib, "statsig_remove_layer_override")

	var statsig_remove_all_overrides func(uint64)
	purego.RegisterLibFunc(&statsig_remove_all_overrides, lib, "statsig_remove_all_overrides")

	// Metadata
	var statsig_metadata_update_values func(string, string, string, string)
	purego.RegisterLibFunc(&statsig_metadata_update_values, lib, "statsig_metadata_update_values")

	// Utility
	var free_string func(string)
	purego.RegisterLibFunc(&free_string, lib, "free_string")

	instance = &StatsigFFI{
		lib: lib,

		// StatsigOptions
		statsig_options_create_from_data: statsig_options_create_from_data,
		statsig_options_release:          statsig_options_release,

		// StatsigUser
		statsig_user_create:  statsig_user_create,
		statsig_user_release: statsig_user_release,

		// Statsig Core
		statsig_create:                           statsig_create,
		statsig_release:                          statsig_release,
		statsig_initialize:                       statsig_initialize,
		statsig_initialize_with_details:          statsig_initialize_with_details,
		statsig_initialize_with_details_blocking: statsig_initialize_with_details_blocking,
		statsig_initialize_blocking:              statsig_initialize_blocking,
		statsig_shutdown:                         statsig_shutdown,
		statsig_shutdown_blocking:                statsig_shutdown_blocking,
		statsig_flush_events:                     statsig_flush_events,
		statsig_flush_events_blocking:            statsig_flush_events_blocking,
		statsig_get_current_values:               statsig_get_current_values,
		statsig_log_event:                        statsig_log_event,
		statsig_identify:                         statsig_identify,
		statsig_get_client_init_response:         statsig_get_client_init_response,

		// Parameter Store
		statsig_get_parameter_store_with_options:           statsig_get_parameter_store_with_options,
		statsig_get_string_parameter_from_parameter_store:  statsig_get_string_parameter_from_parameter_store,
		statsig_get_bool_parameter_from_parameter_store:    statsig_get_bool_parameter_from_parameter_store,
		statsig_get_float64_parameter_from_parameter_store: statsig_get_float64_parameter_from_parameter_store,
		statsig_get_int_parameter_from_parameter_store:     statsig_get_int_parameter_from_parameter_store,
		statsig_get_object_parameter_from_parameter_store:  statsig_get_object_parameter_from_parameter_store,
		statsig_get_array_parameter_from_parameter_store:   statsig_get_array_parameter_from_parameter_store,

		// Gates
		statsig_check_gate:                 statsig_check_gate,
		statsig_check_gate_performance:     statsig_check_gate_performance,
		statsig_get_feature_gate:           statsig_get_feature_gate,
		statsig_manually_log_gate_exposure: statsig_manually_log_gate_exposure,

		// Dynamic Configs
		statsig_get_dynamic_config:                   statsig_get_dynamic_config,
		statsig_manually_log_dynamic_config_exposure: statsig_manually_log_dynamic_config_exposure,

		// Experiments
		statsig_get_experiment:                   statsig_get_experiment,
		statsig_manually_log_experiment_exposure: statsig_manually_log_experiment_exposure,

		// Layers
		statsig_get_layer:                             statsig_get_layer,
		statsig_log_layer_param_exposure:              statsig_log_layer_param_exposure,
		statsig_manually_log_layer_parameter_exposure: statsig_manually_log_layer_parameter_exposure,

		// Prompts
		statsig_get_prompt: statsig_get_prompt,

		// Overrides
		statsig_override_gate:                     statsig_override_gate,
		statsig_override_dynamic_config:           statsig_override_dynamic_config,
		statsig_override_experiment:               statsig_override_experiment,
		statsig_override_experiment_by_group_name: statsig_override_experiment_by_group_name,
		statsig_override_layer:                    statsig_override_layer,
		statsig_remove_gate_override:              statsig_remove_gate_override,
		statsig_remove_dynamic_config_override:    statsig_remove_dynamic_config_override,
		statsig_remove_experiment_override:        statsig_remove_experiment_override,
		statsig_remove_layer_override:             statsig_remove_layer_override,
		statsig_remove_all_overrides:              statsig_remove_all_overrides,

		// Metadata
		statsig_metadata_update_values: statsig_metadata_update_values,

		// Utility
		free_string: free_string,
	}

	instance.updateStatsigMetadata()

	return instance
}

func (ffi *StatsigFFI) updateStatsigMetadata() {
	ffi.statsig_metadata_update_values(
		"statsig-server-core-go", // sdk_type
		runtime.GOOS,             // os
		runtime.GOARCH,           // arch
		runtime.Version(),        // language_version
	)
}

func loadLibrary() (uintptr, error) {
	flags := purego.RTLD_NOW | purego.RTLD_GLOBAL

	// Load from STATSIG_LIB_PATH if set
	override_path := os.Getenv("STATSIG_LIB_PATH")
	if override_path != "" {
		handle, err := purego.Dlopen(override_path, flags)
		if err != nil {
			return 0, fmt.Errorf("STATSIG_LIB_PATH is set but could not be loaded: %w", err)
		}
		return handle, nil
	}

	// Otherwise, load from embedded binary
	path, err := writeBinaryToTempFile()
	if err != nil {
		return 0, fmt.Errorf("failed to write binary to temp file: %w", err)
	}

	handle, err := purego.Dlopen(*path, flags)
	if err != nil {
		return 0, fmt.Errorf("failed to open library: %w", err)
	}

	if err := os.Remove(*path); err != nil {
		return 0, fmt.Errorf("failed to remove temp file: %w", err)
	}

	return handle, nil
}

func writeBinaryToTempFile() (*string, error) {
	binaryData := internal.GetLibData()

	file, err := os.CreateTemp("", "statsiglib")
	if err != nil {
		return nil, fmt.Errorf("failed to create temp file: %w", err)
	}

	defer file.Close()

	_, err = file.Write(binaryData)
	if err != nil {
		return nil, fmt.Errorf("failed to write binary to temp file: %w", err)
	}

	path := file.Name()
	return &path, nil
}
