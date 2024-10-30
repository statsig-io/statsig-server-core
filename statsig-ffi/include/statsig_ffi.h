typedef int Statsig;
typedef int StatsigOptions;
typedef int StatsigUser;

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

const char *statsig_create(const char *sdk_key, const char *options_ref);

void statsig_release(const char *statsig_ref);

void statsig_initialize(const char *statsig_ref, void (*callback)(void));

void statsig_flush_events(const char *statsig_ref, void (*callback)(void));

const char *statsig_get_current_values(const char *statsig_ref);

bool statsig_check_gate(const char *statsig_ref, const char *user_ref, const char *gate_name);

const char *statsig_get_feature_gate(const char *statsig_ref,
                                     const char *user_ref,
                                     const char *gate_name);

const char *statsig_get_dynamic_config(const char *statsig_ref,
                                       const char *user_ref,
                                       const char *config_name);

const char *statsig_get_experiment(const char *statsig_ref,
                                   const char *user_ref,
                                   const char *experiment_name);

const char *statsig_get_layer(const char *statsig_ref,
                              const char *user_ref,
                              const char *layer_name);

void statsig_log_layer_param_exposure(const char *statsig_ref,
                                      const char *layer_json,
                                      const char *param_name);

const char *statsig_get_client_init_response(const char *statsig_ref, const char *user_ref);

const char *statsig_options_create(const char *specs_url, const char *log_event_url);

void statsig_options_release(const char *options_ref);

const char *statsig_user_create(const char *user_id,
                                const char *custom_ids_json,
                                const char *email,
                                const char *ip,
                                const char *user_agent,
                                const char *country,
                                const char *locale,
                                const char *app_version,
                                const char *custom_json,
                                const char *private_attributes_json);

void statsig_user_release(const char *user_ref);
