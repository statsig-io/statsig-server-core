package statsig

type FeatureGate struct {
	Name              string            `json:"name"`
	Value             bool              `json:"value"`
	RuleID            string            `json:"rule_id"`
	EvaluationDetails EvaluationDetails `json:"details"`
	IdType            string            `json:"id_type"`
}
