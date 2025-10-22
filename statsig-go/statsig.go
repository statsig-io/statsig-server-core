package statsig_go_core

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync/atomic"

	"log"
)

type EventPayload struct {
	EventName string `json:"name"`
	// string, int or float
	Value    any               `json:"value"`
	Metadata map[string]string `json:"metadata"`
}

type Statsig struct {
	ref atomic.Uint64
}

func NewStatsig(sdkKey string) (*Statsig, error) {
	ref := GetFFI().statsig_create(sdkKey, 0)
	if ref == 0 {
		return nil, fmt.Errorf("error creating Statsig instance")
	}

	s := &Statsig{ref: atomic.Uint64{}}
	s.ref.Store(ref)

	runtime.SetFinalizer(s, func(obj *Statsig) {
		obj.release()
	})

	return s, nil
}

func NewStatsigWithOptions(sdkKey string, opts *StatsigOptions) (*Statsig, error) {
	ref := GetFFI().statsig_create(sdkKey, opts.ref)
	if ref == 0 {
		return nil, fmt.Errorf("error creating Statsig instance")
	}

	s := &Statsig{ref: atomic.Uint64{}}
	s.ref.Store(ref)

	runtime.SetFinalizer(s, func(obj *Statsig) {
		obj.release()
	})

	return s, nil
}

func (s *Statsig) Initialize() {
	GetFFI().statsig_initialize_blocking(s.ref.Load())
}

func (s *Statsig) Shutdown() {
	GetFFI().statsig_shutdown_blocking(s.ref.Load())
}

func (s *Statsig) FlushEvents() {
	GetFFI().statsig_flush_events_blocking(s.ref.Load())
}

func (s *Statsig) LogEvent(user *StatsigUser, event EventPayload) {
	eventJson, err := json.Marshal(event)
	if err != nil {
		log.Printf("Failed to marshal Statsig event: %v", err)
		return
	}

	GetFFI().statsig_log_event(s.ref.Load(), user.ref, string(eventJson))
}

func (s *Statsig) CheckGate(user *StatsigUser, gateName string) bool {
	return s.CheckGateWithOptions(user, gateName, nil)
}

func (s *Statsig) CheckGateWithOptions(user *StatsigUser, gateName string, options *FeatureGateEvaluationOptions) bool {
	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		fmt.Printf("Failed to marshal FeatureGateEvaluationOptions: %v", err)
		return false
	}

	return GetFFI().statsig_check_gate(s.ref.Load(), user.ref, gateName, optionsJson)
}

func (s *Statsig) GetFeatureGate(user *StatsigUser, gateName string) FeatureGate {
	return s.GetFeatureGateWithOptions(user, gateName, nil)
}

func (s *Statsig) GetFeatureGateWithOptions(user *StatsigUser, gateName string, options *FeatureGateEvaluationOptions) FeatureGate {
	gate := FeatureGate{
		Name: gateName,
	}

	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		fmt.Printf("Failed to marshal FeatureGateEvaluationOptions: %v", err)
		return gate
	}

	gateJson := GetFFI().statsig_get_feature_gate(s.ref.Load(), user.ref, gateName, optionsJson)

	if err := json.Unmarshal([]byte(gateJson), &gate); err != nil {
		fmt.Printf("Failed to unmarshal FeatureGate: %v", err)
	}

	return gate
}

func (s *Statsig) GetDynamicConfig(user *StatsigUser, configName string) DynamicConfig {
	return s.GetDynamicConfigWithOptions(user, configName, nil)
}

func (s *Statsig) GetDynamicConfigWithOptions(user *StatsigUser, configName string, options *DynamicConfigEvaluationOptions) DynamicConfig {
	config := DynamicConfig{
		Name: configName,
	}

	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		fmt.Printf("Failed to marshal DynamicConfigEvaluationOptions: %v", err)
		return config
	}

	configJson := GetFFI().statsig_get_dynamic_config(s.ref.Load(), user.ref, configName, optionsJson)
	if err := json.Unmarshal([]byte(configJson), &config); err != nil {
		fmt.Printf("Failed to unmarshal DynamicConfig: %v", err)
	}

	return config
}

func (s *Statsig) GetExperiment(user *StatsigUser, experimentName string) Experiment {
	return s.GetExperimentWithOptions(user, experimentName, nil)
}

func (s *Statsig) GetExperimentWithOptions(user *StatsigUser, experimentName string, options *ExperimentEvaluationOptions) Experiment {
	experiment := Experiment{
		Name: experimentName,
	}

	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		fmt.Printf("Failed to marshal ExperimentEvaluationOptions: %v", err)
		return experiment
	}

	experimentJson := GetFFI().statsig_get_experiment(s.ref.Load(), user.ref, experimentName, optionsJson)
	if err := json.Unmarshal([]byte(experimentJson), &experiment); err != nil {
		fmt.Printf("Failed to unmarshal Experiment: %v", err)
	}

	return experiment
}

func (s *Statsig) GetLayer(user *StatsigUser, layerName string) Layer {
	return s.GetLayerWithOptions(user, layerName, nil)

}

func (s *Statsig) GetLayerWithOptions(user *StatsigUser, layerName string, options *LayerEvaluationOptions) Layer {
	layer := Layer{
		Name:       layerName,
		statsigRef: s.ref.Load(),
	}

	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		fmt.Printf("Failed to marshal LayerEvaluationOptions: %v", err)
		return layer
	}

	layerJson := GetFFI().statsig_get_layer(s.ref.Load(), user.ref, layerName, optionsJson)
	if err := json.Unmarshal([]byte(layerJson), &layer); err != nil {
		fmt.Printf("Failed to unmarshal Layer: %v", err)
	}

	return layer
}

func (s *Statsig) GetClientInitResponse(user *StatsigUser) *string {
	return s.GetClientInitResponseWithOptions(user, nil)
}

func (s *Statsig) GetClientInitResponseWithOptions(user *StatsigUser, options *ClientInitResponseOptions) *string {
	optionsJson, err := tryMarshalOrEmpty(options)

	if err != nil {
		fmt.Printf("Failed to marshal ClientInitResponseOptions: %v", err)
		return nil
	}

	resp := GetFFI().statsig_get_client_init_response(s.ref.Load(), user.ref, optionsJson)
	return &resp
}

func (s *Statsig) ManuallyLogFeatureGateExposure(user *StatsigUser, gateName string) {
	GetFFI().statsig_manually_log_gate_exposure(s.ref.Load(), user.ref, gateName)
}

func (s *Statsig) ManuallyLogDynamicConfigExposure(user *StatsigUser, configName string) {
	GetFFI().statsig_manually_log_dynamic_config_exposure(s.ref.Load(), user.ref, configName)
}

func (s *Statsig) ManuallyLogExperimentExposure(user *StatsigUser, experimentName string) {
	GetFFI().statsig_manually_log_experiment_exposure(s.ref.Load(), user.ref, experimentName)
}

func (s *Statsig) ManuallyLogLayerParamExposure(user *StatsigUser, layerName string, paramName string) {
	GetFFI().statsig_manually_log_layer_parameter_exposure(s.ref.Load(), user.ref, layerName, paramName)
}


func (s *Statsig) release() {
	was := s.ref.Swap(0)
	if was == 0 {
		return
	}

	GetFFI().statsig_release(was)
}

func tryMarshalOrEmpty[T any](data *T) (string, error) {
	if data == nil {
		return "", nil
	}
	raw, err := json.Marshal(data)
	if err != nil {
		return "", err
	}
	return string(raw), nil
}
