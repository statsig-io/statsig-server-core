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

	gateJson := UseRustString(func() (*byte, uint64) {
		len := uint64(0)
		ptr := GetFFI().statsig_get_raw_feature_gate(s.ref.Load(), user.ref, gateName, optionsJson, &len)
		return ptr, len
	})

	if err := json.Unmarshal([]byte(*gateJson), &gate); err != nil {
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

	configJson := UseRustString(func() (*byte, uint64) {
		len := uint64(0)
		ptr := GetFFI().statsig_get_raw_dynamic_config(s.ref.Load(), user.ref, configName, optionsJson, &len)
		return ptr, len
	})

	if err := json.Unmarshal([]byte(*configJson), &config); err != nil {
		fmt.Printf("Failed to unmarshal DynamicConfig: %v", err)
	}

	if config.Value == nil {
		config.Value = make(map[string]any)
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

	experimentJson := UseRustString(func() (*byte, uint64) {
		len := uint64(0)
		ptr := GetFFI().statsig_get_raw_experiment(s.ref.Load(), user.ref, experimentName, optionsJson, &len)
		return ptr, len
	})

	if err := json.Unmarshal([]byte(*experimentJson), &experiment); err != nil {
		fmt.Printf("Failed to unmarshal Experiment: %v", err)
	}

	if experiment.Value == nil {
		experiment.Value = make(map[string]any)
	}

	return experiment
}

// getExperimentByGroup runs an experiment group-targeting lookup via the given
// FFI call and unmarshals the resulting JSON into an Experiment. Both public
// group getters share this; they differ only in which FFI symbol they invoke.
func (s *Statsig) getExperimentByGroup(
	experimentName string,
	ffiCall func(ref uint64, resultLen *uint64) *byte,
) Experiment {
	experiment := Experiment{
		Name: experimentName,
	}

	experimentJson := UseRustString(func() (*byte, uint64) {
		length := uint64(0)
		ptr := ffiCall(s.ref.Load(), &length)
		return ptr, length
	})

	if experimentJson != nil {
		if err := json.Unmarshal([]byte(*experimentJson), &experiment); err != nil {
			fmt.Printf("Failed to unmarshal Experiment: %v", err)
		}
	}

	if experiment.Value == nil {
		experiment.Value = make(map[string]any)
	}

	return experiment
}

// GetExperimentByGroupName returns the experiment for the rule whose group name
// matches groupName. This is a pure spec lookup and performs no user evaluation,
// so no exposure is logged.
func (s *Statsig) GetExperimentByGroupName(experimentName string, groupName string) Experiment {
	return s.getExperimentByGroup(experimentName, func(ref uint64, resultLen *uint64) *byte {
		return GetFFI().statsig_get_raw_experiment_by_group_name(ref, experimentName, groupName, resultLen)
	})
}

// GetExperimentByGroupIDAdvanced returns the experiment for the rule whose ID
// matches groupID. This is a pure spec lookup and performs no user evaluation,
// so no exposure is logged.
func (s *Statsig) GetExperimentByGroupIDAdvanced(experimentName string, groupID string) Experiment {
	return s.getExperimentByGroup(experimentName, func(ref uint64, resultLen *uint64) *byte {
		return GetFFI().statsig_get_raw_experiment_by_group_id_advanced(ref, experimentName, groupID, resultLen)
	})
}

// OverrideExperimentByGroupName forces an experiment to resolve to a named group.
// When id is nil the override applies globally; otherwise it applies only to the
// provided id (user id or custom id).
func (s *Statsig) OverrideExperimentByGroupName(experimentName string, groupName string, id *string) {
	GetFFI().statsig_override_experiment_by_group_name(s.ref.Load(), experimentName, groupName, cString(id))
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

	layerJson := UseRustString(func() (*byte, uint64) {
		len := uint64(0)
		ptr := GetFFI().statsig_get_raw_layer(s.ref.Load(), user.ref, layerName, optionsJson, &len)
		return ptr, len
	})

	if err := json.Unmarshal([]byte(*layerJson), &layer); err != nil {
		fmt.Printf("Failed to unmarshal Layer: %v", err)
	}

	if layer.value == nil {
		layer.value = make(map[string]any)
	}

	return layer
}

func (s *Statsig) GetParameterStore(user *StatsigUser, storeName string) ParameterStore {
	return s.GetParameterStoreWithOptions(user, storeName, nil)
}

func (s *Statsig) GetParameterStoreWithOptions(
	user *StatsigUser,
	storeName string,
	options *ParameterStoreEvaluationOptions,
) ParameterStore {
	store := ParameterStore{
		Name:       storeName,
		statsigRef: s.ref.Load(),
		options:    options,
	}
	if user != nil {
		store.userRef = user.ref
	}

	optionsJson, err := tryMarshalOrEmpty(options)
	if err != nil {
		fmt.Printf("Failed to marshal ParameterStoreEvaluationOptions: %v", err)
		return store
	}

	storeJson := UseRustString(func() (*byte, uint64) {
		length := uint64(0)
		ptr := GetFFI().statsig_get_parameter_store_with_options(
			s.ref.Load(),
			storeName,
			optionsJson,
			&length,
		)
		return ptr, length
	})
	if storeJson != nil {
		if err := json.Unmarshal([]byte(*storeJson), &store); err != nil {
			fmt.Printf("Failed to unmarshal ParameterStore: %v", err)
		}
	}

	if store.Name == "" {
		store.Name = storeName
	}
	store.statsigRef = s.ref.Load()
	store.options = options
	if user != nil {
		store.userRef = user.ref
	}

	return store
}

// GetFeatureGateList returns the names of all configured feature gates.
func (s *Statsig) GetFeatureGateList() []string {
	return s.getEntityList(GetFFI().statsig_get_feature_gate_list)
}

// GetDynamicConfigList returns the names of all configured dynamic configs.
func (s *Statsig) GetDynamicConfigList() []string {
	return s.getEntityList(GetFFI().statsig_get_dynamic_config_list)
}

// GetExperimentList returns the names of all configured experiments.
func (s *Statsig) GetExperimentList() []string {
	return s.getEntityList(GetFFI().statsig_get_experiment_list)
}

// GetAutotuneList returns the names of all configured autotunes.
func (s *Statsig) GetAutotuneList() []string {
	return s.getEntityList(GetFFI().statsig_get_autotune_list)
}

// GetParameterStoreList returns the names of all configured parameter stores.
func (s *Statsig) GetParameterStoreList() []string {
	return s.getEntityList(GetFFI().statsig_get_parameter_store_list)
}

func (s *Statsig) getEntityList(ffiFn func(uint64, *uint64) *byte) []string {
	list := []string{}

	listJson := UseRustString(func() (*byte, uint64) {
		length := uint64(0)
		ptr := ffiFn(s.ref.Load(), &length)
		return ptr, length
	})

	if listJson == nil {
		return list
	}

	if err := json.Unmarshal([]byte(*listJson), &list); err != nil {
		fmt.Printf("Failed to unmarshal entity list: %v", err)
	}

	return list
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

	return UseRustString(func() (*byte, uint64) {
		len := uint64(0)
		ptr := GetFFI().statsig_get_client_init_response_with_inout_len(s.ref.Load(), user.ref, optionsJson, &len)
		return ptr, len
	})

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
