typedef int Statsig;
typedef int StatsigOptions;
typedef int StatsigUser;

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct StatsigOptionsRef {
  uintptr_t pointer;
} StatsigOptionsRef;

typedef struct StatsigUserRef {
  uintptr_t pointer;
} StatsigUserRef;

typedef struct StatsigRef {
  uintptr_t pointer;
} StatsigRef;

typedef struct Pot {
  uintptr_t pointer;
} Pot;

struct StatsigOptionsRef statsig_options_create(void);

void statsig_options_release(struct StatsigOptionsRef *options_ref);

struct StatsigUserRef statsig_user_create(const char *user_id,
                                          const char *custom_ids_json,
                                          const char *email,
                                          const char *ip,
                                          const char *user_agent,
                                          const char *country,
                                          const char *locale,
                                          const char *app_version,
                                          const char *custom_json,
                                          const char *private_attributes_json);

void statsig_user_release(struct StatsigUserRef *user_ref);

struct StatsigRef statsig_create(const char *sdk_key, struct StatsigOptionsRef options_ref);

void statsig_release(struct StatsigRef *statsig_ref);

void statsig_initialize(struct StatsigRef statsig_ref, void (*callback)(void));

const char *statsig_get_current_values(struct StatsigRef statsig_ref);

bool statsig_check_gate(int64_t statsig_ptr, int64_t user_ptr);

const char *statsig_get_experiment(struct StatsigRef statsig_ref,
                                   struct StatsigUserRef user_ref,
                                   const char *experiment_name);

const char *statsig_get_client_init_response(struct StatsigRef statsig_ref,
                                             struct StatsigUserRef user_ref);

uintptr_t statsig_get_client_init_response_buffer(struct StatsigRef statsig_ref,
                                                  struct StatsigUserRef user_ref,
                                                  char *buffer,
                                                  uintptr_t buffer_size);

struct Pot test_create(void);

void test_mut_star(struct Pot *pot);

void test_value(struct Pot pot);
