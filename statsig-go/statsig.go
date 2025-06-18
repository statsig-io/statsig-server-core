package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"errors"
	"runtime"

	"github.com/statsig-io/private-statsig-server-core/statsig-go/utils"
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
	statsigRef := C.statsig_create(C.CString(key), C.ulonglong(options.innerRef))

	if statsigRef == 0 {
		return nil, errors.New("error occurred in creating Statsig instance")
	}

	s := &Statsig{
		InnerRef: uint64(statsigRef),
	}

	runtime.SetFinalizer(s, func(obj *Statsig) {
		C.statsig_release(C.ulonglong(obj.InnerRef))
	})

	return s, nil

}

func (s *Statsig) Initialize() (bool, error) {
	C.statsig_initialize_blocking(C.ulonglong(s.InnerRef))

	// TODO: return false if statsig_initialize_blocking is updated to return a value
	return true, nil

}

func (s *Statsig) InitializeWithDetails() (InitializeWithDetails, error) {

	res := C.statsig_initialize_with_details(C.ulonglong(s.InnerRef))

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
	C.statsig_shutdown_blocking(C.ulonglong(s.InnerRef))

}

func (s *Statsig) GetFeatureGate(user StatsigUser, gateName string, featureGateOptions *CheckGateOptions) FeatureGate {

	var featureGate FeatureGate

	if featureGateOptions == nil {
		featureGateOptions = &CheckGateOptions{}
	}

	featureGateJson := C.statsig_get_feature_gate(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(gateName), C.CString(utils.ConvertJSONToString(featureGateOptions)))

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

	checkGate := C.statsig_check_gate(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(gateName), C.CString(utils.ConvertJSONToString(gateOptions)))
	return bool(checkGate)
}

func (s *Statsig) GetDynamicConfig(user StatsigUser, configName string, dynamicConfigOptions *GetDynamicConfigOptions) DynamicConfig {

	var dynamicConfig DynamicConfig

	if dynamicConfigOptions == nil {
		dynamicConfigOptions = &GetDynamicConfigOptions{}
	}

	dynamicConfigJson := C.statsig_get_dynamic_config(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(configName), C.CString(utils.ConvertJSONToString(dynamicConfigOptions)))

	if dynamicConfigJson != nil {
		err := json.Unmarshal([]byte(C.GoString(dynamicConfigJson)), &dynamicConfig)

		if err != nil {
			return DynamicConfig{}
		}
	}

	return dynamicConfig
}

func (s *Statsig) GetExperiment(user StatsigUser, experimentName string, experimentOptions *GetExperimentOptions) Experiment {
	var experiment Experiment
	var secondaryExposures []SecondaryExposure

	if experimentOptions == nil {
		experimentOptions = &GetExperimentOptions{}
	}

	experimentJson := C.statsig_get_experiment(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(experimentName), C.CString(utils.ConvertJSONToString(experimentOptions)))

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

func (s *Statsig) GetLayer(user StatsigUser, layerName string, layerOptions *GetLayerOptions) Layer {

	var layer Layer

	if layerOptions == nil {
		layerOptions = &GetLayerOptions{}
	}

	layerJson := C.statsig_get_layer(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(layerName), C.CString(utils.ConvertJSONToString(layerOptions)))

	if layerJson != nil {
		err := json.Unmarshal([]byte(C.GoString(layerJson)), &layer)
		if err != nil {
			return Layer{}
		}

		layer.setStatsigInstance(s)
		layer.setDisableExposureLogging(layerOptions != nil && layerOptions.DisableExposureLogging)
	}

	return layer

}

func (s *Statsig) FlushEvents() {
	C.statsig_flush_events_blocking(C.ulonglong(s.InnerRef))
}

func (s *Statsig) LogEvent(user StatsigUser, eventJson map[string]interface{}) {
	C.statsig_log_event(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(utils.ConvertJSONToString(eventJson)))
}

func (s *Statsig) GetClientInitializeResponse(user StatsigUser, gcirOptions *ClientInitResponseOptions) string {

	if gcirOptions == nil {
		gcirOptions = &ClientInitResponseOptions{}
	}

	res := C.statsig_get_client_init_response(C.ulonglong(s.InnerRef), C.ulonglong(user.innerRef), C.CString(utils.ConvertJSONToString(gcirOptions)))
	return C.GoString(res)
}
