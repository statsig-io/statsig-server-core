package statsig_go_core

type ClientInitResponseOptions struct {
	HashAlgorithm           *string   `json:"hash_algorithm,omitempty"`
	ClientSDKKey            *string   `json:"client_sdk_key,omitempty"`
	IncludeLocalOverrides   *bool     `json:"include_local_overrides,omitempty"`
	ResponseFormat          *string   `json:"response_format,omitempty"`
	FeatureGateFilter       *[]string `json:"feature_gate_filter,omitempty"`
	ExperimentFilter        *[]string `json:"experiment_filter,omitempty"`
	DynamicConfigFilter     *[]string `json:"dynamic_config_filter,omitempty"`
	LayerFilter             *[]string `json:"layer_filter,omitempty"`
	ParamStoreFilter        *[]string `json:"param_store_filter,omitempty"`
	RemoveIdType            *bool     `json:"remove_id_type,omitempty"`
	RemoveDefaultValueGates *bool     `json:"remove_default_value_gates,omitempty"`
}
