from typing import Any, Dict, List, Optional
from typing_extensions import TypeAlias, TypedDict
from statsig_python_core import StatsigUser, PersistentStorageBaseClass

class StickyValues(TypedDict):
    value: bool
    json_value: Optional[Any]
    rule_id: str
    group_name: Optional[str]
    secondary_exposures: List[Dict[str, str]]
    explicit_parameters: Optional[List[str]]
    config_delegate: Optional[str]
    undelegated_secondary_exposures: List[Dict[str, str]]
    config_version: Optional[int]
    time: int

UserPersistedValues: TypeAlias = Dict[str, StickyValues]
PersistedValues: TypeAlias = Dict[str, UserPersistedValues]

class PersistentStorage(PersistentStorageBaseClass):
    def __new__(cls, *args, **kwargs):
        return super().__new__(cls)

    def __init__(self, *args, **kwargs):
        super().__init__()
        self.load_fn = self.load
        self.save_fn = self.save
        self.delete_fn = self.delete

    def load(self, key: str) -> Optional[UserPersistedValues]:
        pass

    def save(self, key: str, config_name: str, data: StickyValues):
        pass

    def delete(self, key: str, config_name: str):
        pass
    

    def get_user_persisted_value(self, user: StatsigUser, id_type: str) -> Optional[UserPersistedValues]:
        storage_key = self.get_storage_key(user, id_type) 
        if storage_key is not None:
            return self.load(storage_key)

        return None
    
    @staticmethod
    def get_storage_key(user: StatsigUser, id_type: str) -> Optional[str]:
        lower_case_id_type = id_type.lower()
        if (lower_case_id_type == "user_id" or lower_case_id_type == "userid"):
            id = getattr(user,"user_id")
            return f"{id}:userID"
        else:
            if (user.custom_ids is not None):
                id = user.custom_ids.get(id_type)
                return f"{id}:{id_type}"
        return None