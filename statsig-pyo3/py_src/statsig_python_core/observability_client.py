from statsig_python_core import ObservabilityClientBase
from typing import Optional, Dict, Tuple


class ObservabilityClient(ObservabilityClientBase):
    def __init__(self):
        super().__init__()
        self.init_fn = self.init
        self.increment_fn = self.increment
        self.gauge_fn = self.gauge
        self.dist_fn = self.dist
        self.error_fn = self.error

    def init(self):
        pass

    def increment(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None):
        pass

    def gauge(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None):
        pass

    def dist(self, metric_name: str, value: float, tags: Optional[Dict[str, str]] = None):
        pass

    def error(self, tag: str, error: str):
        pass
