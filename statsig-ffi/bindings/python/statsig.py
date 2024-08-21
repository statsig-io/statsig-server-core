from _statsig_ffi import ffi, lib

class User:
    def __init__(self, name: str, email: str):
        self._user = lib.create_user(name.encode('utf-8'), email.encode('utf-8'))

    def __del__(self):
        if hasattr(self, '_user'):
            lib.destroy_user(self._user)


class Statsig:
    def __init__(self, sdk_key: str):
        self._statsig = lib.initialize_statsig(sdk_key.encode('utf-8'))

    def __del__(self):
        if hasattr(self, '_statsig'):
            lib.destroy_statsig(self._statsig)

    def check_gate(self, user: User, gate_name: str) -> bool:
        return lib.check_gate(self._statsig, user._user, gate_name.encode('utf-8'))

    def get_client_init_response(self, user: User) -> str:
        res = lib.statsig_get_client_init_response(self._statsig, user._user)
        return ffi.string(res).decode('utf-8')
