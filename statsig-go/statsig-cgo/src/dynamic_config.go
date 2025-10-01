package statsig

import "github.com/statsig-io/statsig-server-core/statsig-go/src/utils"

type DynamicConfig struct {
	Name              string                 `json:"name"`
	Value             map[string]interface{} `json:"value"`
	RuleID            string                 `json:"rule_id"`
	EvaluationDetails EvaluationDetails      `json:"details"`
	IdType            string                 `json:"id_type"`
}

func (d *DynamicConfig) GetString(key string, fallback string) string {
	return utils.GetTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetNumber(key string, fallback float64) float64 {
	return utils.GetTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetBool(key string, fallback bool) bool {
	return utils.GetTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetSlice(key string, fallback []interface{}) []interface{} {
	return utils.GetTypedValue(d.Value, key, fallback)
}

func (d *DynamicConfig) GetMap(key string, fallback map[string]interface{}) map[string]interface{} {
	return utils.GetTypedValue(d.Value, key, fallback)
}
