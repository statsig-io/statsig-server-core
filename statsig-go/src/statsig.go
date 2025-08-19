package statsig

/*
#include "statsig_ffi.h"

#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"errors"

	"runtime"

	"github.com/statsig-io/statsig-server-core/statsig-go/src/utils"
)

type Statsig struct {
	InnerRef uint64
}
type InitializeWithDetails struct {
	Duration          float64         `json:"duration"`
	InitSuccess       bool            `json:"init_success"`
	IsConfigSpecReady bool            `json:"is_config_spec_ready"`
	IsIdListReady     *bool           `json:"is_id_list_ready"`
	Source            string          `json:"source"`
	FailureDetails    *FailureDetails `json:"failure_details"`
}

type FailureDetails struct {
	Reason string                 `json:"reason"`
	Error  map[string]interface{} `json:"error"`
}

func NewStatsig(key string, options StatsigOptions) (*Statsig, error) {
	statsigRef := C.statsig_create(C.CString(key), C.uint64_t(options.innerRef))

	if statsigRef == 0 {
		return nil, errors.New("error occurred in creating Statsig instance")
	}

	s := &Statsig{
		InnerRef: uint64(statsigRef),
	}

	runtime.SetFinalizer(s, func(obj *Statsig) {
		C.statsig_release(C.uint64_t(obj.InnerRef))
	})

	return s, nil

}

func (s *Statsig) Initialize() (bool, error) {
	C.statsig_initialize_blocking(C.uint64_t(s.InnerRef))

	// TODO: return false if statsig_initialize_blocking is updated to return a value
	return true, nil

}

func (s *Statsig) InitializeWithDetails() (InitializeWithDetails, error) {
	res := C.statsig_initialize_with_details_blocking(C.uint64_t(s.InnerRef))

	var details InitializeWithDetails

	if res != nil {
		res_str := C.GoString(res)
		err := json.Unmarshal([]byte(res_str), &details)
		if err != nil {
			return InitializeWithDetails{}, err
		}
	}

	return details, nil
}

func (s *Statsig) Shutdown() {
	C.statsig_shutdown_blocking(C.uint64_t(s.InnerRef))

}

func (s *Statsig) GetFeatureGate(user StatsigUser, gateName string, featureGateOptions *CheckGateOptions) FeatureGate {
	var featureGate FeatureGate

	if featureGateOptions == nil {
		featureGateOptions = &CheckGateOptions{}
	}

	featureGateJson := C.statsig_get_feature_gate(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(gateName), C.CString(utils.ConvertJSONToString(featureGateOptions)))

	if featureGateJson != nil {
		err := json.Unmarshal([]byte(C.GoString(featureGateJson)), &featureGate)
		if err != nil {
			return FeatureGate{}
		}
	}

	return featureGate

}

func (s *Statsig) CheckGate(user StatsigUser, gateName string, gateOptions *CheckGateOptions) bool {
	if gateOptions == nil {
		gateOptions = &CheckGateOptions{}
	}

	checkGate := C.statsig_check_gate(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(gateName), C.CString(utils.ConvertJSONToString(gateOptions)))
	return bool(checkGate)
}

func (s *Statsig) ManuallyLogGateExposure(user StatsigUser, name string) {
	C.statsig_manually_log_gate_exposure(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(name))
}

func (s *Statsig) GetDynamicConfig(user StatsigUser, configName string, dynamicConfigOptions *GetDynamicConfigOptions) DynamicConfig {
	var dynamicConfig DynamicConfig

	if dynamicConfigOptions == nil {
		dynamicConfigOptions = &GetDynamicConfigOptions{}
	}

	dynamicConfigJson := C.statsig_get_dynamic_config(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(configName), C.CString(utils.ConvertJSONToString(dynamicConfigOptions)))

	if dynamicConfigJson != nil {
		err := json.Unmarshal([]byte(C.GoString(dynamicConfigJson)), &dynamicConfig)

		if err != nil {
			return DynamicConfig{}
		}
	}

	return dynamicConfig
}

func (s *Statsig) ManuallyLogDynamicConfigExposure(user StatsigUser, name string) {
	C.statsig_manually_log_dynamic_config_exposure(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(name))
}

func (s *Statsig) GetExperiment(user StatsigUser, experimentName string, experimentOptions *GetExperimentOptions) Experiment {
	var experiment Experiment
	var secondaryExposures []SecondaryExposure

	if experimentOptions == nil {
		experimentOptions = &GetExperimentOptions{}
	}

	experimentJson := C.statsig_get_experiment(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(experimentName), C.CString(utils.ConvertJSONToString(experimentOptions)))

	if experimentJson != nil {

		var combinedRes struct {
			Experiment
			Evaluation map[string]interface{} `json:"__evaluation"`
		}

		err := json.Unmarshal([]byte(C.GoString(experimentJson)), &combinedRes)
		if err != nil {
			return Experiment{}
		}

		experiment = combinedRes.Experiment
		evaluation := combinedRes.Evaluation

		val, _ := json.Marshal(evaluation["secondary_exposures"])
		err = json.Unmarshal(val, &secondaryExposures)

		if err == nil {
			experiment.SecondaryExposures = secondaryExposures
		}

	}
	return experiment
}

func (s *Statsig) ManuallyLogExperimentExposure(user StatsigUser, name string) {
	C.statsig_manually_log_experiment_exposure(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(name))
}

func (s *Statsig) GetLayer(user StatsigUser, layerName string, layerOptions *GetLayerOptions) Layer {
	var layer Layer

	if layerOptions == nil {
		layerOptions = &GetLayerOptions{}
	}

	layerJson := C.statsig_get_layer(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(layerName), C.CString(utils.ConvertJSONToString(layerOptions)))

	if layerJson != nil {
		err := json.Unmarshal([]byte(C.GoString(layerJson)), &layer)
		if err != nil {
			return Layer{}
		}

		layer.setStatsigInstance(s)
		layer.setDisableExposureLogging(layerOptions != nil && layerOptions.DisableExposureLogging)
		layer.setRawResult(C.GoString(layerJson))
	}

	return layer

}

func (s *Statsig) GetPrompt(user StatsigUser, promptName string, layerOptions *GetLayerOptions) Layer {
	var layer Layer

	if layerOptions == nil {
		layerOptions = &GetLayerOptions{}
	}

	layerJson := C.statsig_get_prompt(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(promptName), C.CString(utils.ConvertJSONToString(layerOptions)))

	if layerJson != nil {
		err := json.Unmarshal([]byte(C.GoString(layerJson)), &layer)
		if err != nil {
			return Layer{}
		}

		layer.setStatsigInstance(s)
		layer.setDisableExposureLogging(layerOptions != nil && layerOptions.DisableExposureLogging)
		layer.setRawResult(C.GoString(layerJson))
	}

	return layer

}

func (s *Statsig) ManuallyLogLayerParameterExposure(user StatsigUser, layerName string, paramName string) {
	C.statsig_manually_log_layer_parameter_exposure(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(layerName), C.CString(paramName))
}

func (s *Statsig) FlushEvents() {
	C.statsig_flush_events_blocking(C.uint64_t(s.InnerRef))
}

func (s *Statsig) LogEvent(user StatsigUser, eventJson map[string]interface{}) {
	C.statsig_log_event(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(utils.ConvertJSONToString(eventJson)))
}

func (s *Statsig) GetClientInitializeResponse(user StatsigUser, gcirOptions *ClientInitResponseOptions) string {
	if gcirOptions == nil {
		gcirOptions = &ClientInitResponseOptions{}
	}

	res := C.statsig_get_client_init_response(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(utils.ConvertJSONToString(gcirOptions)))
	return C.GoString(res)
}

func (s *Statsig) GetParameterStore(user StatsigUser, paramStoreName string, paramStoreOptions *ParameterStoreOptions) ParameterStore {
	var paramStore ParameterStore

	if paramStoreOptions == nil {
		paramStoreOptions = &ParameterStoreOptions{}
	}

	paramStoreJson := C.statsig_get_parameter_store_with_options(C.uint64_t(s.InnerRef), C.CString(paramStoreName), C.CString(utils.ConvertJSONToString(paramStoreOptions)))

	if paramStoreJson != nil {
		err := json.Unmarshal([]byte(C.GoString(paramStoreJson)), &paramStore)

		if err != nil {
			return ParameterStore{}
		}

		paramStore.setStatsigInstance(s)
		paramStore.setStatsigUser(&user)
		paramStore.setDisableExposureLogging(paramStoreOptions != nil && paramStoreOptions.DisableExposureLogging)
	}

	return paramStore
}

func (s *Statsig) GetStringFromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal string, psOptions *ParameterStoreOptions) string {
	return C.GoString(C.statsig_get_string_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.CString(defaultVal), C.CString(utils.ConvertJSONToString(psOptions))))
}

func (s *Statsig) GetBooleanFromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal bool, psOptions *ParameterStoreOptions) bool {
	return bool(C.statsig_get_bool_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.int(utils.ConvertToSafeOptBool(&defaultVal)), C.CString(utils.ConvertJSONToString(psOptions))))
}

func (s *Statsig) GetFloat64FromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal float64, psOptions *ParameterStoreOptions) float64 {
	return float64(C.statsig_get_float64_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.double(defaultVal), C.CString(utils.ConvertJSONToString(psOptions))))
}

func (s *Statsig) GetInt64FromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal int64, psOptions *ParameterStoreOptions) int64 {
	return int64(C.statsig_get_int_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.int64_t(defaultVal), C.CString(utils.ConvertJSONToString(psOptions))))
}

func (s *Statsig) GetIntFromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal int, psOptions *ParameterStoreOptions) int {
	return int(C.statsig_get_int_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.int64_t(defaultVal), C.CString(utils.ConvertJSONToString(psOptions))))
}

func (s *Statsig) GetMapFromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal map[string]interface{}, psOptions *ParameterStoreOptions) map[string]interface{} {
	val := C.statsig_get_object_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.CString(utils.ConvertJSONToString(defaultVal)), C.CString(utils.ConvertJSONToString(psOptions)))

	if val != nil {
		paramStoreMap, err := utils.ConvertStringToJSON[map[string]interface{}](C.GoString(val))
		if err == nil {
			return paramStoreMap
		}
	}

	return defaultVal
}

func (s *Statsig) GetInterfaceFromParameterStore(user StatsigUser, paramStoreName string, paramName string, defaultVal []interface{}, psOptions *ParameterStoreOptions) []interface{} {
	val := C.statsig_get_object_parameter_from_parameter_store(C.uint64_t(s.InnerRef), C.uint64_t(user.innerRef), C.CString(paramStoreName), C.CString(paramName), C.CString(utils.ConvertJSONToString(defaultVal)), C.CString(utils.ConvertJSONToString(psOptions)))

	if val != nil {
		paramStoreMap, err := utils.ConvertStringToJSON[[]interface{}](C.GoString(val))
		if err == nil {
			return paramStoreMap
		}
	}

	return defaultVal
}

func (s *Statsig) OverrideGate(gateName string, gateValue bool, id string) {
	C.statsig_override_gate(C.uint64_t(s.InnerRef), C.CString(gateName), C.bool(gateValue), C.CString(id))
}

func (s *Statsig) OverrideLayer(layerName string, layerValue map[string]interface{}, id string) {
	C.statsig_override_layer(C.uint64_t(s.InnerRef), C.CString(layerName), C.CString(utils.ConvertJSONToString(layerValue)), C.CString(id))
}

func (s *Statsig) OverrideDynamicConfig(configName string, configValue map[string]interface{}, id string) {
	C.statsig_override_dynamic_config(C.uint64_t(s.InnerRef), C.CString(configName), C.CString(utils.ConvertJSONToString(configValue)), C.CString(id))
}

func (s *Statsig) OverrideExperiment(experimentName string, experimentValue map[string]interface{}, id string) {
	C.statsig_override_experiment(C.uint64_t(s.InnerRef), C.CString(experimentName), C.CString(utils.ConvertJSONToString(experimentValue)), C.CString(id))
}

func (s *Statsig) OverrideExperimentByGroupName(experimentName string, groupName string, id string) {
	C.statsig_override_experiment_by_group_name(C.uint64_t(s.InnerRef), C.CString(experimentName), C.CString(groupName), C.CString(id))
}

func (s *Statsig) RemoveGateOverride(gateName string, id string) {
	C.statsig_remove_gate_override(C.uint64_t(s.InnerRef), C.CString(gateName), C.CString(id))
}

func (s *Statsig) RemoveDynamicConfigOverride(configName string, id string) {
	C.statsig_remove_dynamic_config_override(C.uint64_t(s.InnerRef), C.CString(configName), C.CString(id))
}

func (s *Statsig) RemoveExperimentOverride(experimentName string, id string) {
	C.statsig_remove_experiment_override(C.uint64_t(s.InnerRef), C.CString(experimentName), C.CString(id))
}

func (s *Statsig) RemoveLayerOverride(layerName string, id string) {
	C.statsig_remove_layer_override(C.uint64_t(s.InnerRef), C.CString(layerName), C.CString(id))
}

func (s *Statsig) RemoveAllOverrides() {
	C.statsig_remove_all_overrides(C.uint64_t(s.InnerRef))
}
