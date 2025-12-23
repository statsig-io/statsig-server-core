package statsig_go_core

import (
	"encoding/json"
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
