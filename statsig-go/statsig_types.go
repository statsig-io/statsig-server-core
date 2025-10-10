package statsig_go_core

import "encoding/json"

// ------------------------------------------------------------------------------------- [ Feature Gate ]

type FeatureGate struct {
	Name   string `json:"name"`
	Value  bool   `json:"value"`
	RuleID string `json:"rule_id"`
	// EvaluationDetails EvaluationDetails `json:"details"`
	IdType string `json:"id_type"`
}

// ------------------------------------------------------------------------------------- [ Dynamic Config ]

type DynamicConfig struct {
	Name   string         `json:"name"`
	Value  map[string]any `json:"value"`
	RuleID string         `json:"rule_id"`
	// EvaluationDetails EvaluationDetails      `json:"details"`
	IdType string `json:"id_type"`
}

func (d *DynamicConfig) GetString(key string, fallback string) string {
	return getTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetNumber(key string, fallback float64) float64 {
	return getTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetBool(key string, fallback bool) bool {
	return getTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetSlice(key string, fallback []any) []any {
	return getTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetMap(key string, fallback map[string]any) map[string]any {
	return getTypedValue(d.Value, key, fallback)
}

// ------------------------------------------------------------------------------------- [ Experiment ]

type Experiment struct {
	Name   string         `json:"name"`
	Value  map[string]any `json:"value"`
	RuleID string         `json:"rule_id"`
	IdType string         `json:"id_type"`
}

func (e *Experiment) GetString(key string, fallback string) string {
	return getTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetNumber(key string, fallback float64) float64 {
	return getTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetBool(key string, fallback bool) bool {
	return getTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetSlice(key string, fallback []any) []any {
	return getTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetMap(key string, fallback map[string]any) map[string]any {
	return getTypedValue(e.Value, key, fallback)
}

// ------------------------------------------------------------------------------------- [ Layer ]

type Layer struct {
	Name   string `json:"name"`
	RuleID string `json:"rule_id"`
	IdType string `json:"id_type"`

	value map[string]any
}

func (l *Layer) GetString(key string, fallback string) string {
	return getTypedValue(l.value, key, fallback)
}

func (l *Layer) GetNumber(key string, fallback float64) float64 {
	return getTypedValue(l.value, key, fallback)
}

func (l *Layer) GetBool(key string, fallback bool) bool {
	return getTypedValue(l.value, key, fallback)
}

func (l *Layer) GetSlice(key string, fallback []any) []any {
	return getTypedValue(l.value, key, fallback)
}

func (l *Layer) GetMap(key string, fallback map[string]any) map[string]any {
	return getTypedValue(l.value, key, fallback)
}

func (l *Layer) UnmarshalJSON(b []byte) error {
	tmp := struct {
		Name   string         `json:"name"`
		RuleID string         `json:"rule_id"`
		IdType string         `json:"id_type"`
		Value  map[string]any `json:"__value"`
	}{}

	if err := json.Unmarshal(b, &tmp); err != nil {
		return err
	}

	l.Name = tmp.Name
	l.RuleID = tmp.RuleID
	l.IdType = tmp.IdType
	l.value = tmp.Value
	return nil
}

// -------------------------------------------------- [ Helper ]

func getTypedValue[T any](values map[string]any, key string, fallback T) T {
	if v, ok := values[key]; ok {
		if typedVal, ok := v.(T); ok {
			return typedVal
		}
	}
	return fallback
}
