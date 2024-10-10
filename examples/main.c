#include <stdio.h>

#ifndef STATSIG_FFI_H
#define STATSIG_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct CUser CUser;
typedef struct Statsig Statsig;

CUser* create_user(const char* user_id, const char* email);
void destroy_user(CUser* user);

Statsig* initialize_statsig(const char* sdk_key);
void destroy_statsig(Statsig* statsig);
int check_gate(Statsig* statsig, CUser* user, const char* gate_name);
const char* statsig_get_client_init_response(Statsig* statsig, CUser* user);

#ifdef __cplusplus
}
#endif

#endif // STATSIG_FFI_H


int main() {
    const char* name = "Dan Smith";
    const char* email = "daniel@statsig.com";

    CUser* user = create_user(name, email);

    const char* sdk_key = getenv("test_api_key");
    Statsig* statsig = initialize_statsig(sdk_key);

    const char* gate_name = "test_public";
    if (check_gate(statsig, user, gate_name)) {
        printf("Gate check passed!\n");
    } else {
        printf("Gate check failed.\n");
    }

    destroy_statsig(statsig);
    destroy_user(user);

    return 0;

}