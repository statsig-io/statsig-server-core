from statsig_python_core import DataStoreBase
from typing import Optional, Dict, Tuple
from dataclasses import dataclass

@dataclass
class DataStoreResponse:
    result: Optional[str]
    time: Optional[int]

class DataStore(DataStoreBase):
    def __init__(self):
        super().__init__()
        self.initialize_fn = self.initialize
        self.shutdown_fn = self.shutdown
        self.get_fn = self.get
        self.set_fn = self.set
        self.support_polling_updates_for_fn = self.support_polling_updates_for

    def initialize(self):
        pass

    def shutdown(self):
        pass

    def get(self, key: str) -> Optional[DataStoreResponse]:
        pass

    def set(self, key: str, value: str, time: Optional[int] = None):
        pass

    def support_polling_updates_for(self, key: str) -> bool:
        return False