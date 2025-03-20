from typing import Optional, Any, Union, Sequence, Mapping, Dict
from typing_extensions import TypeAliasType

class StatsigOptions:
    specs_url: Optional[str]
    specs_sync_interval_ms: Optional[int]
    init_timeout_ms: Optional[int]
    log_event_url: Optional[str]
    disable_all_logging: Optional[bool]
    event_logging_flush_interval_ms: Optional[int]
    event_logging_max_queue_size: Optional[int]
    enable_id_lists: Optional[bool]
    enable_user_agent_parsing: Optional[bool]
    enable_country_lookup: Optional[bool]
    id_lists_url: Optional[str]
    id_lists_sync_interval_ms: Optional[int]
    fallback_to_statsig_api: Optional[bool]
    environment: Optional[str]
    output_log_level: Optional[str]
    observability_client: Optional[ObservabilityClient]

    def __init__(
        self,
        specs_url: Optional[str] = None,
        specs_sync_interval_ms: Optional[int] = None,
        init_timeout_ms: Optional[int] = None,
        log_event_url: Optional[str] = None,
        disable_all_logging: Optional[bool] = None,
        event_logging_flush_interval_ms: Optional[int] = None,
        event_logging_max_queue_size: Optional[int] = None,
        enable_id_lists: Optional[bool] = None,
        enable_user_agent_parsing: Optional[bool] = None,
        enable_country_lookup: Optional[bool] = None,
        id_lists_url: Optional[str] = None,
        id_lists_sync_interval_ms: Optional[int] = None,
        fallback_to_statsig_api: Optional[bool] = None,
        environment: Optional[str] = None,
        output_log_level: Optional[str] = None,
        observability_client: Optional[ObservabilityClient] = None,
    ) -> None: ...

JSONPrimitive = Union[str, int, float, bool, None]
JSONValue: TypeAliasType = Union[
    JSONPrimitive, Sequence["JSONValue"], Mapping[str, "JSONValue"]
]

class StatsigUser:
    """Represents a Statsig user with optional metadata."""

    user_id: Optional[str]
    email: Optional[str]
    ip: Optional[str]
    country: Optional[str]
    locale: Optional[str]
    app_version: Optional[str]
    user_agent: Optional[str]
    custom: Optional[Mapping[str, JSONValue]]
    custom_ids: Optional[Mapping[str, str]]
    private_attributes: Optional[Mapping[str, JSONValue]]

    def __init__(
        self,
        user_id: Optional[str] = None,
        email: Optional[str] = None,
        ip: Optional[str] = None,
        country: Optional[str] = None,
        locale: Optional[str] = None,
        app_version: Optional[str] = None,
        user_agent: Optional[str] = None,
        custom: Optional[Mapping[str, JSONValue]] = None,
        custom_ids: Optional[Mapping[str, str]] = None,
        private_attributes: Optional[Mapping[str, JSONValue]] = None,
    ) -> None:
        """
        Initialize a StatsigUser instance.

        Requires either a UserID or a CustomID to be set
        """
        ...

class FeatureGate:
    name: str
    value: bool
    rule_id: str
    id_type: str

class DynamicConfig:
    name: str
    rule_id: str
    id_type: str
    value: Any

class Experiment:
    name: str
    rule_id: str
    id_type: str
    group_name: Optional[str]
    value: Any

class Layer:
    name: str
    rule_id: str
    group_name: Optional[str]
    allocated_experiment_name: Optional[str]
    value: Any

class DynamicConfigEvaluationOptions:
    disable_exposure_logging: bool

class ExperimentEvaluationOptions:
    disable_exposure_logging: bool

class LayerEvaluationOptions:
    disable_exposure_logging: bool

class FeatureGateEvaluationOptions:
    disable_exposure_logging: bool

class Statsig:
    """
    Statsig SDK.
    """

    def __init__(
        self, secret_key: str, options: Optional[StatsigOptions] = None
    ) -> None: ...
    def initialize(self):
        """
        Initializes the SDK asynchronously.
        """
        ...

    def shutdown(self):
        """
        Shuts down the SDK and releases resources.
        """
        ...

    def flush_events(self):
        """
        Manually trigger flush exposure events operation
        """
        ...

    def check_gate(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[FeatureGateEvaluationOptions] = None,
    ) -> bool:
        """
        :param user: StatsigUser object
        :param name: name of the gate
        :param options: evaluation options, such as disable exposure logging
        :return: bool, whether this user can pass this gate or not
        """
        ...

    def get_feature_gate(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[FeatureGateEvaluationOptions] = None,
    ) -> FeatureGate:
        """
        :param user: StatsigUser Object
        :param name: name of the gate
        :param options: evaluation options, such as disable exposure logging
        :return: FeatureGate: the full feature gate object you are retrieving
        """
        ...

    def manually_log_gate_exposure(
        self,
        user: StatsigUser,
        name: str,
    ) -> None:
        """
        Manually log a gate exposure events
        :param user: StatsigUser Object
        :param name: name of the gate
        """
        ...

    def get_dynamic_config(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[DynamicConfigEvaluationOptions] = None,
    ) -> DynamicConfig: ...

    def manually_log_dynamic_config_exposure(
        self,
        user: StatsigUser,
        name: str,
    ) -> None: ...

    def get_experiment(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[ExperimentEvaluationOptions] = None,
    ) -> Experiment: ...

    def manually_log_experiment_exposure(
        self,
        user: StatsigUser,
        name: str,
    ) -> None: ...

    def get_layer(
        self,
        user: StatsigUser,
        name: str,
        options: Optional[LayerEvaluationOptions] = None,
    ) -> Layer: ...

    def manually_log_layer_parameter_exposure(
        self,
        user: StatsigUser,
        name: str,
        param_name: str,
    ) -> None: ...

    def get_client_initialize_response(
        self,
        user: StatsigUser,
        hash: Optional[str] = None,
        client_sdk_key: Optional[str] = None,
    ) -> str: ...

class ObservabilityClient:
    def init(self) -> None: ...
    def increment(self, metric_name: str, value: int = 1, tags: Optional[Dict[str, str]] = None) -> None: ...
    def gauge(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None) -> None: ...
    def distribution(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None) -> None: ...