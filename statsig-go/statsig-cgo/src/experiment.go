package statsig

import "github.com/statsig-io/statsig-server-core/statsig-go/src/utils"

type Experiment struct {
	Name               string                 `json:"name"`
	Value              map[string]interface{} `json:"value"`
	RuleID             string                 `json:"rule_id"`
	EvaluationDetails  EvaluationDetails      `json:"details"`
	IdType             string                 `json:"id_type"`
	GroupName          string                 `json:"group_name"`
	SecondaryExposures []SecondaryExposure    `json:"__evaluation"`
}

type SecondaryExposure struct {
	Gate      string `json:"gate"`
	GateValue string `json:"gateValue"`
	RuleID    string `json:"ruleID"`
}

func (e *Experiment) GetString(key string, fallback string) string {
	return utils.GetTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetNumber(key string, fallback float64) float64 {
	return utils.GetTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetBool(key string, fallback bool) bool {
	return utils.GetTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetSlice(key string, fallback []interface{}) []interface{} {
	return utils.GetTypedValue(e.Value, key, fallback)
}

func (e *Experiment) GetMap(key string, fallback map[string]interface{}) map[string]interface{} {
	return utils.GetTypedValue(e.Value, key, fallback)
}
