#pragma once
#include "observability_client.h"
#include "persistent_storage.h"
#include <nlohmann/json.hpp>
#include <optional>
#include <string>

using json = nlohmann::json;
namespace statsig_cpp_core {

struct StatsigOptions {
  uint64_t ref;
  StatsigOptions() = default;
  StatsigOptions(const uint64_t ref) { this->ref = ref; }
  ~StatsigOptions();
};

// Configuration for a specs adapter (e.g. gRPC websocket to the Forward Proxy).
// Flattened into spec_adapter_* keys for statsig_options_create_from_data.
// The FFI path configures this one adapter only. If the gRPC proxy is down at
// startup, initialization fails after init_timeout. There is no HTTP specs
// fallback.
struct SpecAdapterConfig {
  static constexpr const char *TYPE_NETWORK_GRPC_WEBSOCKET =
      "network_grpc_websocket";
  static constexpr const char *TYPE_NETWORK_HTTP = "network_http";

  static constexpr const char *AUTH_NONE = "none";
  static constexpr const char *AUTH_TLS = "tls";
  static constexpr const char *AUTH_MTLS = "mtls";

  static constexpr uint64_t DEFAULT_INIT_TIMEOUT_MS = 3000;

  std::string adapter_type;
  std::optional<std::string> specs_url;
  uint64_t init_timeout_ms;
  std::optional<std::string> authentication_mode;
  std::optional<std::string> ca_cert_path;
  std::optional<std::string> client_cert_path;
  std::optional<std::string> client_key_path;
  std::optional<std::string> domain_name;

  // Prefer assigning optional fields by name after construction. Positional
  // order is adapter_type, specs_url, init_timeout_ms, authentication_mode,
  // ca_cert_path, client_cert_path, client_key_path, domain_name.
  //
  // SpecAdapterConfig config(SpecAdapterConfig::TYPE_NETWORK_GRPC_WEBSOCKET);
  // config.specs_url = "http://127.0.0.1:50051";
  // config.authentication_mode = SpecAdapterConfig::AUTH_MTLS;
  // config.ca_cert_path = "/certs/ca.pem";
  // config.client_cert_path = "/certs/client.pem";
  // config.client_key_path = "/certs/client.key";
  // config.domain_name = "proxy.example.com";
  explicit SpecAdapterConfig(
      std::string adapter_type,
      std::optional<std::string> specs_url = std::nullopt,
      std::optional<uint64_t> init_timeout_ms = std::nullopt,
      std::optional<std::string> authentication_mode = std::nullopt,
      std::optional<std::string> ca_cert_path = std::nullopt,
      std::optional<std::string> client_cert_path = std::nullopt,
      std::optional<std::string> client_key_path = std::nullopt,
      std::optional<std::string> domain_name = std::nullopt)
      : adapter_type(std::move(adapter_type)), specs_url(std::move(specs_url)),
        init_timeout_ms(init_timeout_ms.value_or(DEFAULT_INIT_TIMEOUT_MS)),
        authentication_mode(std::move(authentication_mode)),
        ca_cert_path(std::move(ca_cert_path)),
        client_cert_path(std::move(client_cert_path)),
        client_key_path(std::move(client_key_path)),
        domain_name(std::move(domain_name)) {}
};

struct StatsigOptionsBuilder {
public:
  std::optional<std::string> specs_url;
  std::optional<std::string> id_lists_url;
  std::optional<std::string> log_event_url;
  std::optional<std::string> output_log_level;
  std::optional<std::string> environment;
  bool enable_id_lists = false;
  bool enable_dcs_deltas = false;
  bool disable_all_logging = false;
  bool disable_country_lookup = false;
  bool disable_network = false;
  std::optional<uint32_t> exposure_dedupe_max_keys;
  // Non-owning: the caller owns the PersistentStorage and must keep it alive
  // for the lifetime of the resulting Statsig instance. Configuring persistent
  // storage enables persistent assignment (userPersistedValues in the
  // experiment/layer get options).
  std::optional<uint64_t> persistent_storage_ref;
  // Non-owning: the caller owns the ObservabilityClient and must keep it alive
  // for the lifetime of the resulting Statsig instance.
  std::optional<uint64_t> observability_client_ref;
  std::optional<SpecAdapterConfig> spec_adapter_config;
  StatsigOptionsBuilder() = default;
  StatsigOptionsBuilder &
  set_persistent_storage(const PersistentStorage &storage) {
    // A ref of 0 means native registration failed (e.g. slot exhaustion);
    // serialize null rather than an explicit-but-invalid ref.
    if (storage.ref() != 0) {
      persistent_storage_ref = storage.ref();
    } else {
      persistent_storage_ref = std::nullopt;
    }
    return *this;
  }
  StatsigOptionsBuilder &
  set_observability_client(const ObservabilityClient &client) {
    // A ref of 0 means native registration failed (e.g. slot exhaustion);
    // serialize null rather than an explicit-but-invalid ref.
    if (client.ref() != 0) {
      observability_client_ref = client.ref();
    } else {
      observability_client_ref = std::nullopt;
    }
    return *this;
  }
  StatsigOptionsBuilder &
  set_spec_adapter_config(const SpecAdapterConfig &config) {
    spec_adapter_config = config;
    return *this;
  }
  StatsigOptions build();
};

inline void to_json(json &j, const StatsigOptionsBuilder &b) {
  j = json{{"specs_url", b.specs_url},
           {"id_lists_url", b.id_lists_url},
           {"log_event_url", b.log_event_url},
           {"environment", b.environment},
           {"output_log_level", b.output_log_level},
           {"enable_id_lists", b.enable_id_lists},
           {"enable_dcs_deltas", b.enable_dcs_deltas},
           {"disable_all_logging", b.disable_all_logging},
           {"disable_country_lookup", b.disable_country_lookup},
           {"disable_network", b.disable_network},
           {"exposure_dedupe_max_keys", b.exposure_dedupe_max_keys},
           {"persistent_storage_ref", b.persistent_storage_ref},
           {"observability_client_ref", b.observability_client_ref}};
  // Flatten SpecAdapterConfig into the flat spec_adapter_* keys expected by
  // StatsigOptionsData in statsig_options_create_from_data. Both type and
  // init_timeout_ms must be present or the core silently drops the adapter.
  if (b.spec_adapter_config.has_value()) {
    const SpecAdapterConfig &c = b.spec_adapter_config.value();
    j["spec_adapter_type"] = c.adapter_type;
    j["spec_adapter_url"] = c.specs_url;
    j["spec_adapter_init_timeout_ms"] = c.init_timeout_ms;
    j["spec_adapter_authentication_mode"] = c.authentication_mode;
    j["spec_adapter_ca_cert_path"] = c.ca_cert_path;
    j["spec_adapter_client_cert_path"] = c.client_cert_path;
    j["spec_adapter_client_key_path"] = c.client_key_path;
    j["spec_adapter_domain_name"] = c.domain_name;
  }
}

struct CheckGateOptions {
  bool disableExposureLogging;
};

inline void to_json(json &j, const CheckGateOptions &b) {
  j = json{
      {"disable_exposure_logging", b.disableExposureLogging},
  };
}

struct GetDynamicConfigOptions {
  bool disableExposureLogging;
};

inline void to_json(json &j, const GetDynamicConfigOptions &b) {
  j = json{
      {"disable_exposure_logging", b.disableExposureLogging},
  };
}

struct GetExperimentOptions {
  bool disableExposureLogging = false;
  // When a persisted sticky value exists, let a matching console override
  // rule take precedence over it.
  bool enforceOverrides = false;
  // When a persisted sticky value exists, re-check targeting and drop the
  // sticky value if the user no longer passes targeting.
  bool enforceTargeting = false;
  // Map of config name -> sticky values, as loaded through the
  // PersistentStorage load callback. Only honored when a persistent storage
  // adapter is configured on StatsigOptions. std::nullopt means "the caller
  // has no persisted values" and prompts the SDK to delete any active sticky
  // values for this config; pass an empty map for a user with nothing stored
  // when sticky assignment should stay active.
  std::optional<UserPersistedValues> userPersistedValues;
};

inline void to_json(json &j, const GetExperimentOptions &b) {
  j = json{
      {"disable_exposure_logging", b.disableExposureLogging},
      {"enforce_overrides", b.enforceOverrides},
      {"enforce_targeting", b.enforceTargeting},
  };
  if (b.userPersistedValues.has_value()) {
    j["user_persisted_values"] = b.userPersistedValues.value();
  }
}

struct GetLayerOptions {
  bool disableExposureLogging = false;
  // When a persisted sticky value exists, let a matching console override
  // rule take precedence over it.
  bool enforceOverrides = false;
  // When a persisted sticky value exists, re-check targeting and drop the
  // sticky value if the user no longer passes targeting.
  bool enforceTargeting = false;
  // Map of config name -> sticky values, as loaded through the
  // PersistentStorage load callback. Only honored when a persistent storage
  // adapter is configured on StatsigOptions. std::nullopt means "the caller
  // has no persisted values" and prompts the SDK to delete any active sticky
  // values for this config; pass an empty map for a user with nothing stored
  // when sticky assignment should stay active.
  std::optional<UserPersistedValues> userPersistedValues;
};

inline void to_json(json &j, const GetLayerOptions &b) {
  j = json{
      {"disable_exposure_logging", b.disableExposureLogging},
      {"enforce_overrides", b.enforceOverrides},
      {"enforce_targeting", b.enforceTargeting},
  };
  if (b.userPersistedValues.has_value()) {
    j["user_persisted_values"] = b.userPersistedValues.value();
  }
}
}; // namespace statsig_cpp_core
