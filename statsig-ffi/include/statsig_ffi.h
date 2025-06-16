typedef int Statsig;
typedef int StatsigOptions;
typedef int StatsigUser;

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef int SafeOptBool;

uint64_t function_based_event_logging_adapter_create(void (*start_fn)(void),
                                                     SafeOptBool (*log_events_fn)(const char *request_json),
                                                     void (*shutdown_fn)(void));

void function_based_event_logging_adapter_release(uint64_t adapter_ref);

uint64_t function_based_specs_adapter_create(void (*setup_internal_fn)(uint64_t listener_ref),
                                             void (*start_fn)(void),
                                             void (*shutdown_fn)(void),
                                             void (*schedule_background_sync_fn)(void));

void function_based_specs_adapter_release(uint64_t adapter_ref);

void specs_update_listener_release(uint64_t listener_ref);

void specs_update_listener_did_receive_specs_update(uint64_t listener_ref,
                                                    const char *data,
                                                    const char *source,
                                                    unsigned long received_at);

const char *specs_update_listener_get_current_specs_info(uint64_t listener_ref);

uint64_t statsig_create(const char *sdk_key, uint64_t options_ref);

void statsig_release(uint64_t statsig_ref);

void statsig_initialize(uint64_t statsig_ref, void (*callback)(void));

const char *statsig_initialize_with_details(uint64_t statsig_ref);

void statsig_initialize_blocking(uint64_t statsig_ref);

void statsig_shutdown_blocking(uint64_t statsig_ref);

void statsig_flush_events(uint64_t statsig_ref, void (*callback)(void));

void statsig_flush_events_blocking(uint64_t statsig_ref);

const char *statsig_get_current_values(uint64_t statsig_ref);

void statsig_log_event(uint64_t statsig_ref, uint64_t user_ref, const char *event_json);

void statsig_identify(uint64_t statsig_ref, uint64_t user_ref);

const char *statsig_get_client_init_response(uint64_t statsig_ref,
                                             uint64_t user_ref,
                                             const char *options_json);

bool statsig_check_gate(uint64_t statsig_ref,
                        uint64_t user_ref,
                        const char *gate_name,
                        const char *options_json);

const char *statsig_get_feature_gate(uint64_t statsig_ref,
                                     uint64_t user_ref,
                                     const char *gate_name,
                                     const char *options_json);

void statsig_manually_log_gate_exposure(uint64_t statsig_ref,
                                        uint64_t user_ref,
                                        const char *gate_name);

const char *statsig_get_dynamic_config(uint64_t statsig_ref,
                                       uint64_t user_ref,
                                       const char *config_name,
                                       const char *options_json);

void statsig_manually_log_dynamic_config_exposure(uint64_t statsig_ref,
                                                  uint64_t user_ref,
                                                  const char *config_name);

const char *statsig_get_experiment(uint64_t statsig_ref,
                                   uint64_t user_ref,
                                   const char *experiment_name,
                                   const char *options_json);

void statsig_manually_log_experiment_exposure(uint64_t statsig_ref,
                                              uint64_t user_ref,
                                              const char *experiment_name);

const char *statsig_get_layer(uint64_t statsig_ref,
                              uint64_t user_ref,
                              const char *layer_name,
                              const char *options_json);

void statsig_log_layer_param_exposure(uint64_t statsig_ref,
                                      const char *layer_json,
                                      const char *param_name);

void statsig_manually_log_layer_parameter_exposure(uint64_t statsig_ref,
                                                   uint64_t user_ref,
                                                   const char *layer_name,
                                                   const char *param_name);

uint64_t statsig_http_event_logging_adapter_create(const char *sdk_key, uint64_t options_ref);

void statsig_http_event_logging_adapter_release(uint64_t event_logging_adapter_ref);

void statsig_http_event_logging_adapter_send_events(uint64_t event_logging_adapter_ref,
                                                    const char *request_json,
                                                    void (*callback)(bool, const char*));

uint64_t statsig_http_specs_adapter_create(const char *sdk_key, uint64_t options_ref);

void statsig_http_specs_adapter_release(uint64_t specs_adapter_ref);

const char *statsig_http_specs_adapter_fetch_specs_from_network(uint64_t specs_adapter_ref,
                                                                const char *current_specs_info);

uint64_t statsig_local_file_event_logging_adapter_create(const char *sdk_key,
                                                         const char *output_directory,
                                                         const char *log_event_url,
                                                         bool disable_network);

void statsig_local_file_event_logging_adapter_release(uint64_t event_logging_adapter_ref);

void statsig_local_file_event_logging_adapter_send_pending_events(uint64_t adapter_ref);

uint64_t statsig_local_file_specs_adapter_create(const char *sdk_key,
                                                 const char *output_directory,
                                                 const char *specs_url,
                                                 bool fallback_to_statsig_api,
                                                 bool disable_network);

void statsig_local_file_specs_adapter_release(uint64_t specs_adapter_ref);

void statsig_local_file_specs_adapter_fetch_and_write_to_file(uint64_t specs_adapter_ref);

void statsig_metadata_update_values(const char *sdk_type,
                                    const char *os,
                                    const char *arch,
                                    const char *language_version);

uint64_t statsig_options_create(const char *specs_url,
                                const char *log_event_url,
                                uint64_t specs_adapter_ref,
                                uint64_t event_logging_adapter_ref,
                                const char *environment,
                                int _event_logging_flush_interval_ms,
                                int event_logging_max_queue_size,
                                int specs_sync_interval_ms,
                                const char *output_log_level,
                                SafeOptBool disable_country_lookup,
                                SafeOptBool disable_user_agent_parsing,
                                SafeOptBool wait_for_country_lookup_init,
                                SafeOptBool wait_for_user_agent_init,
                                SafeOptBool enable_id_lists,
                                const char *id_lists_url,
                                int id_lists_sync_interval_ms);

void statsig_options_release(uint64_t options_ref);

uint64_t statsig_user_create(const char *user_id,
                             const char *custom_ids_json,
                             const char *email,
                             const char *ip,
                             const char *user_agent,
                             const char *country,
                             const char *locale,
                             const char *app_version,
                             const char *custom_json,
                             const char *private_attributes_json);

void statsig_user_release(uint64_t user_ref);
