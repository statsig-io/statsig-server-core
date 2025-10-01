package statsig

type ClientInitResponseOptions struct {
	HashAlgo              *HashAlgo           `json:"hash_algorithm,omitempty"`
	ClientSDKKey          *string             `json:"client_sdk_key,omitempty"`
	IncludeLocalOverrides *bool               `json:"include_local_overrides,omitempty"`
	ResponseFormat        *GCIRResponseFormat `json:"response_format,omitempty"`
	FeatureGateFilter     *[]string           `json:"feature_gate_filter,omitempty"`
	ExperimentFilter      *[]string           `json:"experiment_filter,omitempty"`
	DynamicConfigFilter   *[]string           `json:"dynamic_config_filter,omitempty"`
	LayerFilter           *[]string           `json:"layer_filter,omitempty"`
}
