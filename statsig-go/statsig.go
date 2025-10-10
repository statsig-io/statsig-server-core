package statsig_go_core

import (
	"encoding/json"

	"log"
)

type EventPayload struct {
	EventName string `json:"name"`
	// string, int or float
	Value    any               `json:"value"`
	Metadata map[string]string `json:"metadata"`
}

type Statsig struct {
	ref uint64
}

func NewStatsig(sdkKey string) *Statsig {
	return &Statsig{
		ref: GetFFI().statsig_create(sdkKey, 0),
	}
}

func NewStatsigWithOptions(sdkKey string, opts *StatsigOptions) *Statsig {
	return &Statsig{
		ref: GetFFI().statsig_create(sdkKey, opts.ref),
	}
}

func (s *Statsig) Initialize() {
	GetFFI().statsig_initialize_blocking(s.ref)
}

func (s *Statsig) Shutdown() {
	GetFFI().statsig_shutdown_blocking(s.ref)
}

func (s *Statsig) FlushEvents() {
	GetFFI().statsig_flush_events_blocking(s.ref)
}

func (s *Statsig) LogEvent(user *StatsigUser, event EventPayload) {
	eventJson, err := json.Marshal(event)
	if err != nil {
		log.Printf("Failed to marshal Statsig event: %v", err)
		return
	}

	GetFFI().statsig_log_event(s.ref, user.ref, string(eventJson))
}

func (s *Statsig) CheckGate(user *StatsigUser, gateName string) bool {
	return GetFFI().statsig_check_gate(s.ref, user.ref, gateName, nil)
}

func (s *Statsig) GetFeatureGate(user *StatsigUser, gateName string) FeatureGate {
	gateJson := GetFFI().statsig_get_feature_gate(s.ref, user.ref, gateName, nil)
	featureGate := FeatureGate{}
	json.Unmarshal([]byte(gateJson), &featureGate)
	return featureGate
}

func (s *Statsig) GetDynamicConfig(user *StatsigUser, configName string) DynamicConfig {
	configJson := GetFFI().statsig_get_dynamic_config(s.ref, user.ref, configName, nil)
	dynamicConfig := DynamicConfig{}
	json.Unmarshal([]byte(configJson), &dynamicConfig)
	return dynamicConfig
}

func (s *Statsig) GetExperiment(user *StatsigUser, experimentName string) Experiment {
	experimentJson := GetFFI().statsig_get_experiment(s.ref, user.ref, experimentName, nil)
	experiment := Experiment{}
	json.Unmarshal([]byte(experimentJson), &experiment)
	return experiment
}

func (s *Statsig) GetLayer(user *StatsigUser, layerName string) Layer {
	layerJson := GetFFI().statsig_get_layer(s.ref, user.ref, layerName, nil)
	layer := Layer{}
	json.Unmarshal([]byte(layerJson), &layer)
	return layer
}
