typedef int Statsig;
typedef int StatsigOptions;
typedef int StatsigUser;

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

const char *function_based_specs_adapter_create(void (*setup_internal_fn)(const char *listener_ref),
                                                void (*start_fn)(void),
                                                void (*shutdown_fn)(void),
                                                void (*schedule_background_sync_fn)(void));

void function_based_specs_adapter_release(const char *adapter_ref);

void specs_update_listener_release(const char *listener_ref);

void specs_update_listener_did_receive_specs_update(const char *listener_ref,
                                                    const char *data,
                                                    const char *source,
                                                    unsigned long received_at);

const char *specs_update_listener_get_current_specs_info(const char *listener_ref);

const char *statsig_create(const char *sdk_key, const char *options_ref);

void statsig_release(const char *statsig_ref);

void statsig_initialize(const char *statsig_ref, void (*callback)(void));

void statsig_initialize_blocking(const char *statsig_ref);

void statsig_shutdown_blocking(const char *statsig_ref);

void statsig_flush_events(const char *statsig_ref, void (*callback)(void));

void statsig_flush_events_blocking(const char *statsig_ref);

const char *statsig_get_current_values(const char *statsig_ref);

void statsig_log_event(const char *statsig_ref, const char *user_ref, const char *event_json);

const char *statsig_get_client_init_response(const char *statsig_ref,
                                             const char *user_ref,
                                             const char *options_json);

bool statsig_check_gate(const char *statsig_ref,
                        const char *user_ref,
                        const char *gate_name,
                        const char *options_json);

const char *statsig_get_feature_gate(const char *statsig_ref,
                                     const char *user_ref,
                                     const char *gate_name,
                                     const char *options_json);

void statsig_manually_log_gate_exposure(const char *statsig_ref,
                                        const char *user_ref,
                                        const char *gate_name);

const char *statsig_get_dynamic_config(const char *statsig_ref,
                                       const char *user_ref,
                                       const char *config_name,
                                       const char *options_json);

void statsig_manually_log_dynamic_config_exposure(const char *statsig_ref,
                                                  const char *user_ref,
                                                  const char *config_name);

const char *statsig_get_experiment(const char *statsig_ref,
                                   const char *user_ref,
                                   const char *experiment_name,
                                   const char *options_json);

void statsig_manually_log_experiment_exposure(const char *statsig_ref,
                                              const char *user_ref,
                                              const char *experiment_name);

const char *statsig_get_layer(const char *statsig_ref,
                              const char *user_ref,
                              const char *layer_name,
                              const char *options_json);

void statsig_log_layer_param_exposure(const char *statsig_ref,
                                      const char *layer_json,
                                      const char *param_name);

void statsig_manually_log_layer_parameter_exposure(const char *statsig_ref,
                                                   const char *user_ref,
                                                   const char *layer_name,
                                                   const char *param_name);

const char *statsig_http_event_logging_adapter_create(const char *sdk_key, const char *options_ref);

void statsig_http_event_logging_adapter_release(const char *event_logging_adapter_ref);

void statsig_http_event_logging_adapter_send_events(const char *event_logging_adapter_ref,
                                                    const char *request_json,
                                                    void (*callback)(bool, const char*));

const char *statsig_http_specs_adapter_create(const char *sdk_key, const char *options_ref);

void statsig_http_specs_adapter_release(const char *specs_adapter_ref);

const char *statsig_http_specs_adapter_fetch_specs_from_network(const char *specs_adapter_ref,
                                                                const char *current_specs_info);

const char *statsig_local_file_event_logging_adapter_create(const char *sdk_key,
                                                            const char *output_directory,
                                                            const char *log_event_url,
                                                            bool disable_network);

void statsig_local_file_event_logging_adapter_release(const char *event_logging_adapter_ref);

void statsig_local_file_event_logging_adapter_send_pending_events(const char *adapter_ref);

const char *statsig_local_file_specs_adapter_create(const char *sdk_key,
                                                    const char *output_directory,
                                                    const char *specs_url,
                                                    bool fallback_to_statsig_api,
                                                    bool disable_network);

void statsig_local_file_specs_adapter_release(const char *specs_adapter_ref);

void statsig_local_file_specs_adapter_fetch_and_write_to_file(const char *specs_adapter_ref);

void statsig_metadata_update_values(const char *sdk_type,
                                    const char *os,
                                    const char *arch,
                                    const char *language_version);

const char *statsig_options_create(const char *specs_url,
                                   const char *log_event_url,
                                   const char *specs_adapter_ref,
                                   const char *event_logging_adapter_ref,
                                   const char *environment,
                                   int _event_logging_flush_interval_ms,
                                   int event_logging_max_queue_size,
                                   int specs_sync_interval_ms,
                                   const char *output_log_level);

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
