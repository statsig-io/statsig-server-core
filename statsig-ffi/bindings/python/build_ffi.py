from cffi import FFI
import shutil
import os

ffi = FFI()

ffi.cdef("""
    typedef struct Ref Statsig;
    typedef struct Ref StatsigUser;

    StatsigUser* create_user(const char* user_id, const char* email);
    void destroy_user(StatsigUser* user);

    Statsig* initialize_statsig(const char* sdk_key);
    void destroy_statsig(Statsig* statsig);
    int check_gate(Statsig* statsig, StatsigUser* user, const char* gate_name);
    const char* statsig_get_client_init_response(Statsig* statsig, StatsigUser* user);
""")

ffi.set_source("_statsig_ffi",
"""
    #include "statsig_ffi.h"
""",
    libraries=["statsig_ffi"],
    library_dirs=["../../../target/release"],
    include_dirs=["../../../statsig-ffi/include"]
)

if __name__ == "__main__":
    ffi.compile(verbose=True, tmpdir="../../../build/python/sigstat")
