import json
from typing import Any, Callable, Optional


def _log_error(tag: str, message: str):
    print(f"[Statsig::{tag}]: {message}")


class EvaluationDetails:
    reason: str
    lcut: Optional[int]
    received_at: Optional[int]
    version: Optional[int]

    def __init__(self, data: Optional[dict]):
        data = data or {}
        try:
            self.reason = data.get("reason") or ""
            self.lcut = data.get("lcut")
            self.received_at = data.get("received_at")
            self.version = data.get("version")
        except Exception as error:
            _log_error("EvaluationDetails", f"Failed to parse. {error}")

    def to_dict(self) -> dict:
        return {
            "reason": self.reason,
            "lcut": self.lcut,
            "received_at": self.received_at,
            "version": self.version,
        }


class BaseEvaluation:
    name: str
    rule_id: str
    id_type: str
    details: EvaluationDetails

    def __init__(self, name: str, data: dict):
        self.name = name
        self.rule_id = data.get("ruleID") or ""
        self.id_type = data.get("idType") or ""
        self.details = EvaluationDetails(data.get("details"))

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "rule_id": self.rule_id,
            "id_type": self.id_type,
            "details": self.details.to_dict(),
        }

    def get_evaluation_details(self) -> EvaluationDetails:
        return self.details

    def get_name(self) -> str:
        return self.name

    def get_rule_id(self) -> str:
        return self.rule_id

    def get_id_type(self) -> str:
        return self.id_type

    def _get_typed(
        self,
        tag: str,
        value: dict,
        key: str,
        fallback: Any,
        exposure_func: Optional[Callable] = None,
    ) -> Any:
        res = value.get(key, None)
        if fallback is not None and not isinstance(fallback, type(res)):
            _log_error(
                tag,
                f"Type mismatch - '{self.name}.{key}'. Expected {type(fallback)}, got {type(res)}",
            )
            return fallback

        if res is not None:
            if exposure_func is not None:
                exposure_func(key)
            return res

        return fallback


class FeatureGate(BaseEvaluation):
    value: bool

    def __init__(self, name: str, raw: str):
        try:
            data = json.loads(raw) or {}
            super().__init__(name, data)
            self.value = data.get("value") or False
        except Exception as error:
            super().__init__(name, {})
            self.value = False
            _log_error("FeatureGate", f"Failed to parse. {error}")

    def to_dict(self) -> dict:
        base_dict = super().to_dict()
        base_dict["value"] = self.value
        return base_dict


class BaseConfigEvaluation(BaseEvaluation):
    value: dict
    __tag: str

    def __init__(self, name: str, data: dict, tag: str):
        self.__tag = tag
        super().__init__(name, data)
        self.value = data.get("value") or {}

    def get_value(self) -> dict:
        return self.value

    def get(self, param_name: str, fallback: Any = None) -> Any:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def get_typed(self, param_name: str, fallback: Any = None) -> Any:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def get_string(self, param_name: str, fallback: str) -> str:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def get_integer(self, param_name: str, fallback: int) -> int:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def get_float(self, param_name: str, fallback: float) -> float:
        actual_fallback = float(fallback) if isinstance(fallback, int) else fallback
        return self._get_typed(self.__tag, self.value, param_name, actual_fallback)

    def get_bool(self, param_name: str, fallback: bool) -> bool:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def get_array(self, param_name: str, fallback: list) -> list:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def get_object(self, param_name: str, fallback: dict) -> dict:
        return self._get_typed(self.__tag, self.value, param_name, fallback)

    def to_dict(self) -> dict:
        base_dict = super().to_dict()
        base_dict["value"] = self.value
        return base_dict


class DynamicConfig(BaseConfigEvaluation):
    def __init__(self, name: str, raw: str):
        try:
            data = json.loads(raw) or {}
            super().__init__(name, data, "DynamicConfig")
        except Exception as error:
            super().__init__(name, {}, "DynamicConfig")
            _log_error("DynamicConfig", f"Failed to parse. {error}")


class Experiment(BaseConfigEvaluation):
    group_name: Optional[str]

    def __init__(self, name: str, raw: str):
        try:
            data = json.loads(raw) or {}
            super().__init__(name, data, "Experiment")
            self.group_name = data.get("groupName")
        except Exception as error:
            super().__init__(name, {}, "Experiment")
            self.group_name = None
            _log_error("Experiment", f"Failed to parse. {error}")

    def to_dict(self) -> dict:
        base_dict = super().to_dict()
        base_dict["group_name"] = self.group_name
        return base_dict


class Layer(BaseEvaluation):
    group_name: Optional[str]
    allocated_experiment_name: Optional[str]
    __value: dict
    __exposure_func: Optional[Callable]

    def __init__(self, exposure_func: Callable, name: str, raw: str):
        try:
            data = json.loads(raw) or {}
            super().__init__(name, data)
            self.group_name = data.get("groupName")
            self.allocated_experiment_name = data.get("allocatedExperimentName")
            self.__value = data.get("value") or {}
            self.__exposure_func = exposure_func
        except Exception as error:
            super().__init__(name, {})
            self.group_name = None
            self.allocated_experiment_name = None
            self.__value = {}
            self.__exposure_func = None
            _log_error("Layer", f"Failed to parse. {error}")

    def get_value(self) -> dict:
        return self.__value

    def get(self, param_name: str, fallback: Any = None) -> Any:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def get_typed(self, param_name: str, fallback: Any = None) -> Any:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def get_string(self, param_name: str, fallback: str) -> str:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def get_integer(self, param_name: str, fallback: int) -> int:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def get_float(self, param_name: str, fallback: float) -> float:
        actual_fallback = float(fallback) if isinstance(fallback, int) else fallback
        return self._get_typed(
            "Layer",
            self.__value,
            param_name,
            actual_fallback,
            self._log_layer_param_exposure,
        )

    def get_bool(self, param_name: str, fallback: bool) -> bool:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def get_array(self, param_name: str, fallback: list) -> list:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def get_object(self, param_name: str, fallback: dict) -> dict:
        return self._get_typed(
            "Layer", self.__value, param_name, fallback, self._log_layer_param_exposure
        )

    def to_dict(self) -> dict:
        base_dict = super().to_dict()
        base_dict["group_name"] = self.group_name
        base_dict["allocated_experiment_name"] = self.allocated_experiment_name
        base_dict["__value"] = self.__value
        return base_dict

    def _log_layer_param_exposure(self, param_name: str):
        if self.__exposure_func is None:
            return

        self.__exposure_func(param_name)
