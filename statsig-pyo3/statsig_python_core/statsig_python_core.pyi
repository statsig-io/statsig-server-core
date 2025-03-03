from typing import Optional

class StatsigOptions: ...
class StatsigUser: ...
class FeatureGate: ...
class Layer: ...
class DynamicConfig: ...
class Experiment: ...
class DynamicConfigEvaluationOptions: ...
class ExperimentEvaluationOptions: ...
class LayerEvaluationOptions: ...
class FeatureGateEvaluationOptions: ...

class Statsig:
    """
    Statsig SDK.
    """

    def __init__(
        self, secret_key: str, options: Optional[StatsigOptions] = None
    ) -> None: ...
    def initialize(self) -> None:
        """
        Initializes the SDK asynchronously.
        """
        ...

    def shutdown(self) -> None:
        """
        Shuts down the SDK and releases resources.
        """
        ...

    def flush_events(self) -> None:
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
