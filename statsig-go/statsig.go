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
	s.release()
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
	return GetFFI().statsig_check_gate(s.ref.Load(), user.ref, gateName, "")
}

func (s *Statsig) GetFeatureGate(user *StatsigUser, gateName string) FeatureGate {
	gateJson := GetFFI().statsig_get_feature_gate(s.ref.Load(), user.ref, gateName, "")
	featureGate := FeatureGate{}
	json.Unmarshal([]byte(gateJson), &featureGate)
	return featureGate
}

func (s *Statsig) GetDynamicConfig(user *StatsigUser, configName string) DynamicConfig {
	configJson := GetFFI().statsig_get_dynamic_config(s.ref.Load(), user.ref, configName, "")
	dynamicConfig := DynamicConfig{}
	json.Unmarshal([]byte(configJson), &dynamicConfig)
	return dynamicConfig
}

func (s *Statsig) GetExperiment(user *StatsigUser, experimentName string) Experiment {
	experimentJson := GetFFI().statsig_get_experiment(s.ref.Load(), user.ref, experimentName, "")
	experiment := Experiment{}
	json.Unmarshal([]byte(experimentJson), &experiment)
	return experiment
}

func (s *Statsig) GetLayer(user *StatsigUser, layerName string) Layer {
	layerJson := GetFFI().statsig_get_layer(s.ref.Load(), user.ref, layerName, "")
	layer := Layer{
		statsigRef: s.ref.Load(),
	}
	json.Unmarshal([]byte(layerJson), &layer)

	return layer
}

func (s *Statsig) GetClientInitResponse(user *StatsigUser, options *ClientInitResponseOptions) (*string, error) {
	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		return nil, err
	}

	resp := GetFFI().statsig_get_client_init_response(s.ref.Load(), user.ref, optionsJson)
	return &resp, nil
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
