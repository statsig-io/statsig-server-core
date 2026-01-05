import os
from typing import Dict, Any, Optional, Union
import threading
from statsig_python_core import (
    Statsig,
    StatsigUser,
    StatsigOptions,
)
from statsig import (
    statsig,
    StatsigUser as LegacyStatsigUser,
    StatsigOptions as LegacyStatsigOptions,
    StatsigEvent,
    HashingAlgorithm,
)

SCRAPI_URL = "http://scrapi:8000"


class StatsigWrapper:
    is_core: bool = False

    _statsig: Optional[Statsig] = None
    _user: Optional[Union[StatsigUser, Dict[str, Any]]] = None

    @classmethod
    def initialize(cls) -> threading.Event:
        variant = os.environ.get("SDK_VARIANT")

        if variant == "core":
            cls.is_core = True

            options = StatsigOptions(
                specs_url=f"{SCRAPI_URL}/v2/download_config_specs",
                log_event_url=f"{SCRAPI_URL}/v1/log_event",
                output_log_level="debug",
                # disable_user_agent_parsing=True,
                # disable_country_lookup=True,
            )

            cls._statsig = Statsig("secret-PYTHON_CORE", options)
            return cls._statsig.initialize()

        if variant == "legacy":
            cls.is_core = False

            options = LegacyStatsigOptions(api=f"{SCRAPI_URL}/v1")
            statsig.initialize("secret-PYTHON_LEGACY", options)

            event = threading.Event()
            event.set()
            return event

        raise ValueError(f"Invalid SDK variant: {variant}")

    @classmethod
    def set_user(cls, user_data: Dict[str, Any]):
        if cls.is_core:
            cls._user = StatsigUser(user_id=user_data["userID"])
        else:
            cls._user = LegacyStatsigUser(user_id=user_data["userID"])

    @classmethod
    def check_gate(cls, gate_name: str) -> bool:
        if cls.is_core:
            cls._validate_core_user()
            return cls._statsig.check_gate(cls._user, gate_name)

        return statsig.check_gate(cls._user, gate_name)

    @classmethod
    def log_event(cls, event_name: str):
        if cls.is_core:
            cls._validate_core_user()
            cls._statsig.log_event(cls._user, event_name)
        else:
            event = StatsigEvent(
                user=cls._user,
                event_name=event_name,
            )
            statsig.log_event(event)

    @classmethod
    def get_client_initialize_response(cls):
        if cls.is_core:
            cls._validate_core_user()
            return cls._statsig.get_client_initialize_response(cls._user)
        else:
            return statsig.get_client_initialize_response(
                cls._user, hash=HashingAlgorithm.DJB2
            )

    @classmethod
    def _validate_core_user(cls):
        if not isinstance(cls._user, StatsigUser):
            raise ValueError("User not set or not a StatsigUser")
