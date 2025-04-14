# This file is automatically generated by pyo3_stub_gen
# ruff: noqa: E501, F401

import builtins
import typing

class DataStoreBase:
    def __new__(cls,): ...
    ...

class DynamicConfig:
    name: builtins.str
    rule_id: builtins.str
    id_type: builtins.str
    value: typing.Any
    details: EvaluationDetails

class DynamicConfigEvaluationOptions:
    disable_exposure_logging: builtins.bool
    def __new__(cls,disable_exposure_logging:builtins.bool=False): ...

class EvaluationDetails:
    reason: builtins.str
    lcut: typing.Optional[builtins.int]
    received_at: typing.Optional[builtins.int]

class Experiment:
    name: builtins.str
    rule_id: builtins.str
    id_type: builtins.str
    group_name: typing.Optional[builtins.str]
    value: typing.Any
    details: EvaluationDetails

class ExperimentEvaluationOptions:
    disable_exposure_logging: builtins.bool
    user_persisted_values: typing.Optional[dict]
    def __new__(cls,disable_exposure_logging:builtins.bool=False, user_persisted_values:typing.Optional[dict]=None): ...

class FeatureGate:
    name: builtins.str
    value: builtins.bool
    rule_id: builtins.str
    id_type: builtins.str
    details: EvaluationDetails

class FeatureGateEvaluationOptions:
    disable_exposure_logging: builtins.bool
    def __new__(cls,disable_exposure_logging:builtins.bool=False): ...

class Layer:
    name: builtins.str
    rule_id: builtins.str
    group_name: typing.Optional[builtins.str]
    allocated_experiment_name: typing.Optional[builtins.str]
    value: typing.Any
    details: EvaluationDetails

class LayerEvaluationOptions:
    disable_exposure_logging: builtins.bool
    user_persisted_values: typing.Optional[dict]
    def __new__(cls,disable_exposure_logging:builtins.bool=False, user_persisted_values:typing.Optional[dict]=None): ...

class ObservabilityClientBase:
    def __new__(cls,): ...
    ...

class PersistentStorageBaseClass:
    def __new__(cls,): ...
    ...

class StatsigBasePy:
    def __new__(cls,network_func:typing.Any, sdk_key:builtins.str, options:typing.Optional[StatsigOptions]=None): ...
    def initialize(self) -> typing.Any:
        ...

    def flush_events(self) -> typing.Any:
        ...

    def shutdown(self) -> typing.Any:
        ...

    def log_event(self, user:StatsigUser, event_name:builtins.str, value:typing.Optional[typing.Any]=None, metadata:typing.Optional[dict]=None) -> None:
        ...

    def check_gate(self, user:StatsigUser, name:builtins.str, options:typing.Optional[FeatureGateEvaluationOptions]=None) -> builtins.bool:
        ...

    def get_feature_gate(self, user:StatsigUser, name:builtins.str, options:typing.Optional[FeatureGateEvaluationOptions]=None) -> FeatureGate:
        ...

    def manually_log_gate_exposure(self, user:StatsigUser, name:builtins.str) -> None:
        ...

    def get_dynamic_config(self, user:StatsigUser, name:builtins.str, options:typing.Optional[DynamicConfigEvaluationOptions]=None) -> DynamicConfig:
        ...

    def manually_log_dynamic_config_exposure(self, user:StatsigUser, name:builtins.str) -> None:
        ...

    def get_experiment(self, user:StatsigUser, name:builtins.str, options:typing.Optional[ExperimentEvaluationOptions]=None) -> Experiment:
        ...

    def manually_log_experiment_exposure(self, user:StatsigUser, name:builtins.str) -> None:
        ...

    def get_layer(self, user:StatsigUser, name:builtins.str, options:typing.Optional[LayerEvaluationOptions]=None) -> Layer:
        ...

    def manually_log_layer_parameter_exposure(self, user:StatsigUser, name:builtins.str, param_name:builtins.str) -> None:
        ...

    def get_client_initialize_response(self, user:StatsigUser, hash:typing.Optional[builtins.str]=None, client_sdk_key:typing.Optional[builtins.str]=None, include_local_overrides:typing.Optional[builtins.bool]=None) -> builtins.str:
        ...

    def override_gate(self, gate_name:builtins.str, value:builtins.bool) -> None:
        ...

    def override_dynamic_config(self, config_name:builtins.str, value:dict) -> None:
        ...

    def override_experiment(self, experiment_name:builtins.str, value:dict) -> None:
        ...

    def override_layer(self, layer_name:builtins.str, value:dict) -> None:
        ...

    def override_experiment_by_group_name(self, experiment_name:builtins.str, group_name:builtins.str) -> None:
        ...


class StatsigOptions:
    specs_url: typing.Optional[builtins.str]
    specs_sync_interval_ms: typing.Optional[builtins.int]
    init_timeout_ms: typing.Optional[builtins.int]
    log_event_url: typing.Optional[builtins.str]
    disable_all_logging: typing.Optional[builtins.bool]
    disable_network: typing.Optional[builtins.bool]
    event_logging_flush_interval_ms: typing.Optional[builtins.int]
    event_logging_max_queue_size: typing.Optional[builtins.int]
    event_logging_max_pending_batch_queue_size: typing.Optional[builtins.int]
    enable_id_lists: typing.Optional[builtins.bool]
    enable_user_agent_parsing: typing.Optional[builtins.bool]
    enable_country_lookup: typing.Optional[builtins.bool]
    id_lists_url: typing.Optional[builtins.str]
    id_lists_sync_interval_ms: typing.Optional[builtins.int]
    fallback_to_statsig_api: typing.Optional[builtins.bool]
    environment: typing.Optional[builtins.str]
    output_log_level: typing.Optional[builtins.str]
    global_custom_fields: typing.Optional[dict]
    observability_client: typing.Optional[ObservabilityClientBase]
    data_store: typing.Optional[DataStoreBase]
    persistent_storage: typing.Optional[PersistentStorageBaseClass]
    def __new__(cls,specs_url:typing.Optional[builtins.str]=None, specs_sync_interval_ms:typing.Optional[builtins.int]=None, init_timeout_ms:typing.Optional[builtins.int]=None, log_event_url:typing.Optional[builtins.str]=None, disable_all_logging:typing.Optional[builtins.bool]=None, disable_network:typing.Optional[builtins.bool]=None, event_logging_flush_interval_ms:typing.Optional[builtins.int]=None, event_logging_max_queue_size:typing.Optional[builtins.int]=None, event_logging_max_pending_batch_queue_size:typing.Optional[builtins.int]=None, enable_id_lists:typing.Optional[builtins.bool]=None, enable_user_agent_parsing:typing.Optional[builtins.bool]=None, enable_country_lookup:typing.Optional[builtins.bool]=None, id_lists_url:typing.Optional[builtins.str]=None, id_lists_sync_interval_ms:typing.Optional[builtins.int]=None, fallback_to_statsig_api:typing.Optional[builtins.bool]=None, environment:typing.Optional[builtins.str]=None, output_log_level:typing.Optional[builtins.str]=None, global_custom_fields:typing.Optional[dict]=None, observability_client:typing.Optional[ObservabilityClientBase]=None, data_store:typing.Optional[DataStoreBase]=None, persistent_storage:typing.Optional[PersistentStorageBaseClass]=None): ...

class StatsigUser:
    user_id: typing.Optional[builtins.str]
    email: typing.Optional[builtins.str]
    ip: typing.Optional[builtins.str]
    country: typing.Optional[builtins.str]
    locale: typing.Optional[builtins.str]
    app_version: typing.Optional[builtins.str]
    user_agent: typing.Optional[builtins.str]
    custom: typing.Optional[dict]
    custom_ids: typing.Optional[dict]
    private_attributes: typing.Optional[dict]
    def __new__(cls,user_id:typing.Optional[builtins.str]=None, email:typing.Optional[builtins.str]=None, ip:typing.Optional[builtins.str]=None, country:typing.Optional[builtins.str]=None, locale:typing.Optional[builtins.str]=None, app_version:typing.Optional[builtins.str]=None, user_agent:typing.Optional[builtins.str]=None, custom:typing.Optional[dict]=None, custom_ids:typing.Optional[dict]=None, private_attributes:typing.Optional[dict]=None): ...

