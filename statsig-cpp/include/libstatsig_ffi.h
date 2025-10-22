#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef int SafeOptBool;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint64_t data_store_create(void (*initialize_fn)(void),
                           void (*shutdown_fn)(void),
                           const char *(*get_fn)(const char *args_ptr, uint64_t args_length),
                           void (*set_fn)(const char *args_ptr, uint64_t args_length),
                           bool (*support_polling_updates_for_fn)(const char *args_ptr,
                                                                  uint64_t args_length));

void data_store_release(uint64_t data_store_ref);

const char *__internal__test_data_store(uint64_t data_store_ref,
                                        const char *path,
                                        const char *value);

void free_string(char *s);

uint64_t function_based_event_logging_adapter_create(void (*start_fn)(void),
                                                     SafeOptBool (*log_events_fn)(const char *request_json),
                                                     void (*shutdown_fn)(void));

void function_based_event_logging_adapter_release(uint64_t adapter_ref);

uint64_t function_based_specs_adapter_create(const char *bindings_language,
                                             void (*setup_internal_fn)(uint64_t listener_ref),
                                             void (*start_fn)(void),
                                             void (*shutdown_fn)(void),
                                             void (*schedule_background_sync_fn)(void));

void function_based_specs_adapter_release(uint64_t adapter_ref);

uint64_t observability_client_create(void (*init_fn)(void),
                                     void (*increment_fn)(const char *args_ptr, uint64_t args_length),
                                     void (*gauge_fn)(const char *args_ptr, uint64_t args_length),
                                     void (*dist_fn)(const char *args_ptr, uint64_t args_length),
                                     void (*error_fn)(const char *args_ptr, uint64_t args_length),
                                     bool (*should_enable_high_cardinality_for_this_tag_fn)(const char *args_ptr,
                                                                                            uint64_t args_length));

void observability_client_release(uint64_t data_store_ref);

void __internal__test_observability_client(uint64_t ob_client_ref,
                                           const char *action,
                                           const char *metric_name,
                                           double value,
                                           const char *tags);

uint64_t persistent_storage_create(char *(*load_fn)(const char *args_ptr, uint64_t args_length),
                                   void (*save_fn)(const char *args_ptr, uint64_t args_length),
                                   void (*delete_fn)(const char *args_ptr, uint64_t args_length));

void persistent_storage_release(uint64_t storage_ref);

char *__internal__test_persistent_storage(uint64_t storage_ref,
                                          const char *action,
                                          const char *key,
                                          const char *config_name,
                                          const char *data);

void specs_update_listener_release(uint64_t listener_ref);

void specs_update_listener_did_receive_specs_update(uint64_t listener_ref,
                                                    const char *data,
                                                    const char *source,
                                                    unsigned long received_at);

char *specs_update_listener_get_current_specs_info(uint64_t listener_ref);

uint64_t statsig_create(const char *sdk_key, uint64_t options_ref);

void statsig_release(uint64_t statsig_ref);

void statsig_initialize(uint64_t statsig_ref, void (*callback)(void));

void statsig_initialize_with_details(uint64_t statsig_ref, void (*callback)(char *result));

char *statsig_initialize_with_details_blocking(uint64_t statsig_ref);

void statsig_initialize_blocking(uint64_t statsig_ref);

void statsig_shutdown(uint64_t statsig_ref, void (*callback)(void));

void statsig_shutdown_blocking(uint64_t statsig_ref);

void statsig_flush_events(uint64_t statsig_ref, void (*callback)(void));

void statsig_flush_events_blocking(uint64_t statsig_ref);

char *statsig_get_current_values(uint64_t statsig_ref);

void statsig_log_event(uint64_t statsig_ref, uint64_t user_ref, const char *event_json);

void statsig_identify(uint64_t statsig_ref, uint64_t user_ref);

char *statsig_get_client_init_response(uint64_t statsig_ref,
                                       uint64_t user_ref,
                                       const char *options_json);

char *statsig_get_parameter_store_with_options(uint64_t statsig_ref,
                                               const char *parameter_store_name,
                                               const char *options_json);

char *statsig_get_string_parameter_from_parameter_store(uint64_t statsig_ref,
                                                        uint64_t user_ref,
                                                        const char *parameter_store_name,
                                                        const char *param_name,
                                                        char *default_value,
                                                        const char *options_json);

bool statsig_get_bool_parameter_from_parameter_store(uint64_t statsig_ref,
                                                     uint64_t user_ref,
                                                     const char *parameter_store_name,
                                                     const char *param_name,
                                                     SafeOptBool default_,
                                                     const char *options_json);

double statsig_get_float64_parameter_from_parameter_store(uint64_t statsig_ref,
                                                          uint64_t user_ref,
                                                          const char *parameter_store_name,
                                                          const char *param_name,
                                                          double default_,
                                                          const char *options_json);

int64_t statsig_get_int_parameter_from_parameter_store(uint64_t statsig_ref,
                                                       uint64_t user_ref,
                                                       const char *parameter_store_name,
                                                       const char *param_name,
                                                       int64_t default_,
                                                       const char *options_json);

const char *statsig_get_object_parameter_from_parameter_store(uint64_t statsig_ref,
                                                              uint64_t user_ref,
                                                              const char *parameter_store_name,
                                                              const char *param_name,
                                                              const char *default_,
                                                              const char *options_json);

const char *statsig_get_array_parameter_from_parameter_store(uint64_t statsig_ref,
                                                             uint64_t user_ref,
                                                             const char *parameter_store_name,
                                                             const char *param_name,
                                                             const char *default_,
                                                             const char *options_json);

bool statsig_check_gate(uint64_t statsig_ref,
                        uint64_t user_ref,
                        const char *gate_name,
                        const char *options_json);

bool statsig_check_gate_performance(uint64_t statsig_ref,
                                    uint64_t user_ref,
                                    const uint8_t *gate_name_ptr,
                                    uintptr_t gate_len,
                                    const uint8_t *options_ptr,
                                    uintptr_t options_len);

char *statsig_get_feature_gate(uint64_t statsig_ref,
                               uint64_t user_ref,
                               const char *gate_name,
                               const char *options_json);

void statsig_manually_log_gate_exposure(uint64_t statsig_ref,
                                        uint64_t user_ref,
                                        const char *gate_name);

char *statsig_get_dynamic_config(uint64_t statsig_ref,
                                 uint64_t user_ref,
                                 const char *config_name,
                                 const char *options_json);

void statsig_manually_log_dynamic_config_exposure(uint64_t statsig_ref,
                                                  uint64_t user_ref,
                                                  const char *config_name);

char *statsig_get_experiment(uint64_t statsig_ref,
                             uint64_t user_ref,
                             const char *experiment_name,
                             const char *options_json);

void statsig_manually_log_experiment_exposure(uint64_t statsig_ref,
                                              uint64_t user_ref,
                                              const char *experiment_name);

char *statsig_get_layer(uint64_t statsig_ref,
                        uint64_t user_ref,
                        const char *layer_name,
                        const char *options_json);

char *statsig_get_prompt(uint64_t statsig_ref,
                         uint64_t user_ref,
                         const char *prompt_name,
                         const char *options_json);

void statsig_log_layer_param_exposure(uint64_t statsig_ref,
                                      const char *layer_json,
                                      const char *param_name);

void statsig_manually_log_layer_parameter_exposure(uint64_t statsig_ref,
                                                   uint64_t user_ref,
                                                   const char *layer_name,
                                                   const char *param_name);

void statsig_override_gate(uint64_t statsig_ref, const char *gate_name, bool value, const char *id);

void statsig_override_dynamic_config(uint64_t statsig_ref,
                                     const char *config_name,
                                     const char *value_json,
                                     const char *id);

void statsig_override_experiment(uint64_t statsig_ref,
                                 const char *experiment_name,
                                 const char *value_json,
                                 const char *id);

void statsig_override_experiment_by_group_name(uint64_t statsig_ref,
                                               const char *experiment_name,
                                               const char *group_name,
                                               const char *id);

void statsig_override_layer(uint64_t statsig_ref,
                            const char *layer_name,
                            const char *value_json,
                            const char *id);

void statsig_remove_gate_override(uint64_t statsig_ref, const char *gate_name, const char *id);

void statsig_remove_dynamic_config_override(uint64_t statsig_ref,
                                            const char *config_name,
                                            const char *id);

void statsig_remove_experiment_override(uint64_t statsig_ref,
                                        const char *experiment_name,
                                        const char *id);

void statsig_remove_layer_override(uint64_t statsig_ref, const char *layer_name, const char *id);

void statsig_remove_all_overrides(uint64_t statsig_ref);

uint64_t statsig_http_event_logging_adapter_create(const char *sdk_key, uint64_t options_ref);

void statsig_http_event_logging_adapter_release(uint64_t event_logging_adapter_ref);

void statsig_http_event_logging_adapter_send_events(uint64_t event_logging_adapter_ref,
                                                    const char *request_json,
                                                    void (*callback)(bool, const char*));

uint64_t statsig_http_specs_adapter_create(const char *sdk_key, uint64_t options_ref);

void statsig_http_specs_adapter_release(uint64_t specs_adapter_ref);

char *statsig_http_specs_adapter_fetch_specs_from_network(uint64_t specs_adapter_ref,
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

uint64_t statsig_options_create_from_data(const char *json_data);

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
                                SafeOptBool wait_for_country_lookup_init,
                                SafeOptBool wait_for_user_agent_init,
                                SafeOptBool enable_id_lists,
                                SafeOptBool disable_network,
                                const char *id_lists_url,
                                int id_lists_sync_interval_ms,
                                SafeOptBool disable_all_logging,
                                const char *global_custom_fields,
                                uint64_t observability_client_ref,
                                uint64_t data_store_ref,
                                int init_timeout_ms,
                                SafeOptBool fallback_to_statsig_api,
                                SafeOptBool use_third_party_ua_parser,
                                const char *proxy_host,
                                int proxy_port,
                                const char *proxy_auth,
                                const char *proxy_protocol,
                                uint64_t persistent_storage_ref);

void statsig_options_release(uint64_t options_ref);

uint64_t statsig_user_create_from_data(const char *json_data);

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

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
