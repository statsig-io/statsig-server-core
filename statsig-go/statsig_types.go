package statsig_go_core

import (
	"encoding/json"
	"fmt"
)

// ------------------------------------------------------------------------------------- [ Evaluation Details ]

type EvaluationDetails struct {
	ReceivedAt uint64 `json:"received_at"`
	LCUT       uint64 `json:"lcut"`
	Reason     string `json:"reason"`
}

// ------------------------------------------------------------------------------------- [ Feature Gate ]

type FeatureGate struct {
	Name              string            `json:"name"`
	Value             bool              `json:"value"`
	RuleID            string            `json:"ruleID"`
	EvaluationDetails EvaluationDetails `json:"details"`
	IDType            string            `json:"idType"`
}

// ------------------------------------------------------------------------------------- [ Dynamic Config ]

type DynamicConfig struct {
	Name              string            `json:"name"`
	Value             map[string]any    `json:"value"`
	RuleID            string            `json:"ruleID"`
	EvaluationDetails EvaluationDetails `json:"details"`
	IDType            string            `json:"idType"`
}

func (d *DynamicConfig) GetString(key string, fallback string) string {
	return getTypedValue(d.Value, key, fallback, nil)
}

func (d *DynamicConfig) GetNumber(key string, fallback float64) float64 {
	return getTypedValue(d.Value, key, fallback, nil)
}

func (d *DynamicConfig) GetBool(key string, fallback bool) bool {
	return getTypedValue(d.Value, key, fallback, nil)
}

func (d *DynamicConfig) GetSlice(key string, fallback []any) []any {
	return getTypedValue(d.Value, key, fallback, nil)
}

func (d *DynamicConfig) GetMap(key string, fallback map[string]any) map[string]any {
	return getTypedValue(d.Value, key, fallback, nil)
}

// ------------------------------------------------------------------------------------- [ Experiment ]

type Experiment struct {
	Name              string            `json:"name"`
	Value             map[string]any    `json:"value"`
	RuleID            string            `json:"ruleID"`
	IDType            string            `json:"idType"`
	GroupName         *string           `json:"groupName,omitempty"`
	EvaluationDetails EvaluationDetails `json:"details"`
}

func (e *Experiment) GetString(key string, fallback string) string {
	return getTypedValue(e.Value, key, fallback, nil)
}

func (e *Experiment) GetNumber(key string, fallback float64) float64 {
	return getTypedValue(e.Value, key, fallback, nil)
}

func (e *Experiment) GetBool(key string, fallback bool) bool {
	return getTypedValue(e.Value, key, fallback, nil)
}

func (e *Experiment) GetSlice(key string, fallback []any) []any {
	return getTypedValue(e.Value, key, fallback, nil)
}

func (e *Experiment) GetMap(key string, fallback map[string]any) map[string]any {
	return getTypedValue(e.Value, key, fallback, nil)
}

// ------------------------------------------------------------------------------------- [ Layer ]

type Layer struct {
	Name                    string            `json:"name"`
	RuleID                  string            `json:"ruleID"`
	IDType                  string            `json:"idType"`
	GroupName               *string           `json:"groupName,omitempty"`
	AllocatedExperimentName *string           `json:"allocatedExperimentName,omitempty"`
	EvaluationDetails       EvaluationDetails `json:"details"`

	value      map[string]any
	rawJson    string
	statsigRef uint64
}

func (l *Layer) GetString(key string, fallback string) string {
	return getTypedValue(l.value, key, fallback, l.logExposure)
}

func (l *Layer) GetNumber(key string, fallback float64) float64 {
	return getTypedValue(l.value, key, fallback, l.logExposure)
}

func (l *Layer) GetBool(key string, fallback bool) bool {
	return getTypedValue(l.value, key, fallback, l.logExposure)
}

func (l *Layer) GetSlice(key string, fallback []any) []any {
	return getTypedValue(l.value, key, fallback, l.logExposure)
}

func (l *Layer) GetMap(key string, fallback map[string]any) map[string]any {
	return getTypedValue(l.value, key, fallback, l.logExposure)
}

func (l *Layer) UnmarshalJSON(b []byte) error {
	tmp := struct {
		Name                    string            `json:"name"`
		RuleID                  string            `json:"ruleID"`
		IDType                  string            `json:"idType"`
		Value                   map[string]any    `json:"value"`
		GroupName               *string           `json:"groupName,omitempty"`
		AllocatedExperimentName *string           `json:"allocatedExperimentName,omitempty"`
		EvaluationDetails       EvaluationDetails `json:"details"`
	}{}

	if err := json.Unmarshal(b, &tmp); err != nil {
		return err
	}

	l.Name = tmp.Name
	l.RuleID = tmp.RuleID
	l.IDType = tmp.IDType
	l.value = tmp.Value
	l.GroupName = tmp.GroupName
	l.AllocatedExperimentName = tmp.AllocatedExperimentName
	l.EvaluationDetails = tmp.EvaluationDetails
	l.rawJson = string(b)
	return nil
}

func (l *Layer) logExposure(key string) {
	GetFFI().log_layer_param_exposure_from_raw(l.statsigRef, l.rawJson, key)
}

// ------------------------------------------------------------------------------------- [ Parameter Store ]

type ParameterStore struct {
	Name              string            `json:"name"`
	EvaluationDetails EvaluationDetails `json:"details"`

	statsigRef uint64
	userRef    uint64
	options    *ParameterStoreEvaluationOptions
}

func (p *ParameterStore) GetString(key string, fallback string) string {
	return parameterStoreValue(p, fallback, func(optionsJson string) (string, bool) {
		result := UseRustString(func() (*byte, uint64) {
			length := uint64(0)
			ptr := GetFFI().statsig_get_string_parameter_from_parameter_store(
				p.statsigRef,
				p.userRef,
				p.Name,
				key,
				fallback,
				optionsJson,
				&length,
			)
			return ptr, length
		})
		if result == nil {
			return fallback, false
		}
		return *result, true
	})
}

func (p *ParameterStore) GetBool(key string, fallback bool) bool {
	return parameterStoreValue(p, fallback, func(optionsJson string) (bool, bool) {
		return GetFFI().statsig_get_bool_parameter_from_parameter_store(
			p.statsigRef,
			p.userRef,
			p.Name,
			key,
			safeOptBool(fallback),
			optionsJson,
		), true
	})
}

func (p *ParameterStore) GetNumber(key string, fallback float64) float64 {
	return parameterStoreValue(p, fallback, func(optionsJson string) (float64, bool) {
		return GetFFI().statsig_get_float64_parameter_from_parameter_store(
			p.statsigRef,
			p.userRef,
			p.Name,
			key,
			fallback,
			optionsJson,
		), true
	})
}

func (p *ParameterStore) GetInt(key string, fallback int64) int64 {
	return parameterStoreValue(p, fallback, func(optionsJson string) (int64, bool) {
		return GetFFI().statsig_get_int_parameter_from_parameter_store(
			p.statsigRef,
			p.userRef,
			p.Name,
			key,
			fallback,
			optionsJson,
		), true
	})
}

func (p *ParameterStore) GetMap(key string, fallback map[string]any) map[string]any {
	return parameterStoreValue(p, fallback, func(optionsJson string) (map[string]any, bool) {
		defaultJson, err := json.Marshal(fallback)
		if err != nil {
			return fallback, false
		}

		result := UseRustString(func() (*byte, uint64) {
			length := uint64(0)
			ptr := GetFFI().statsig_get_object_parameter_from_parameter_store(
				p.statsigRef,
				p.userRef,
				p.Name,
				key,
				string(defaultJson),
				optionsJson,
				&length,
			)
			return ptr, length
		})
		if result == nil {
			return fallback, false
		}

		var parsed map[string]any
		if err := json.Unmarshal([]byte(*result), &parsed); err != nil {
			return fallback, false
		}
		return parsed, true
	})
}

func (p *ParameterStore) GetSlice(key string, fallback []any) []any {
	return parameterStoreValue(p, fallback, func(optionsJson string) ([]any, bool) {
		defaultJson, err := json.Marshal(fallback)
		if err != nil {
			return fallback, false
		}

		result := UseRustString(func() (*byte, uint64) {
			length := uint64(0)
			ptr := GetFFI().statsig_get_array_parameter_from_parameter_store(
				p.statsigRef,
				p.userRef,
				p.Name,
				key,
				string(defaultJson),
				optionsJson,
				&length,
			)
			return ptr, length
		})
		if result == nil {
			return fallback, false
		}

		var parsed []any
		if err := json.Unmarshal([]byte(*result), &parsed); err != nil {
			return fallback, false
		}
		return parsed, true
	})
}

func (p *ParameterStore) getOptionsJson() (string, bool) {
	optionsJson, err := tryMarshalOrEmpty(p.options)
	if err != nil {
		fmt.Printf("Failed to marshal ParameterStoreEvaluationOptions: %v", err)
		return "", false
	}
	return optionsJson, true
}

func parameterStoreValue[T any](store *ParameterStore, fallback T, handler func(optionsJson string) (T, bool)) T {
	if store == nil {
		return fallback
	}

	optionsJson, ok := store.getOptionsJson()
	if !ok {
		return fallback
	}

	value, ok := handler(optionsJson)
	if !ok {
		return fallback
	}
	return value
}

func safeOptBool(value bool) int32 {
	if value {
		return 1
	}
	return 0
}

// -------------------------------------------------- [ Helper ]

func getTypedValue[T any](values map[string]any, key string, fallback T, exposureFunc func(string)) T {
	v, ok := values[key]
	if !ok {
		return fallback
	}

	typedVal, ok := v.(T)
	if !ok {
		return fallback
	}

	if exposureFunc != nil {
		exposureFunc(key)
	}
	return typedVal
}
