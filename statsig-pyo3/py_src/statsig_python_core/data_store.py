from statsig_python_core import DataStoreBase
from typing import Optional
from dataclasses import dataclass

@dataclass
class DataStoreResponse:
    result: Optional[str]
    time: Optional[int]


@dataclass
class DataStoreBytesResponse:
    result: Optional[bytes]
    time: Optional[int]


class DataStore(DataStoreBase):
    def __new__(cls, *args, **kwargs):
        return super().__new__(cls)

    def __init__(self, *args, **kwargs):
        super().__init__()
        self.initialize_fn = self.initialize
        self.shutdown_fn = self.shutdown
        self.get_fn = self.get
        self.set_fn = self.set
        self.support_polling_updates_for_fn = self.support_polling_updates_for

        if type(self) is not DataStore and type(self).__dict__.get("get_bytes") is not None:
            self.get_bytes_fn = self.get_bytes
        else:
            self.get_bytes_fn = None

        if type(self) is not DataStore and type(self).__dict__.get("set_bytes") is not None:
            self.set_bytes_fn = self.set_bytes
        else:
            self.set_bytes_fn = None

    def initialize(self):
        pass

    def shutdown(self):
        pass

    def get(self, key: str) -> Optional[DataStoreResponse]:
        pass

    def get_bytes(self, key: str) -> Optional[DataStoreBytesResponse]:
        pass

    def set(self, key: str, value: str, time: Optional[int] = None):
        pass

    def set_bytes(self, key: str, value: bytes, time: Optional[int] = None):
        pass

    def support_polling_updates_for(self, key: str) -> bool:
        return False
