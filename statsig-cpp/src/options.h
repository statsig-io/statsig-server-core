#pragma once
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
  StatsigOptionsBuilder() = default;
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
           {"disable_network", b.disable_network}};
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
  bool disableExposureLogging;
};

inline void to_json(json &j, const GetExperimentOptions &b) {
  j = json{
      {"disable_exposure_logging", b.disableExposureLogging},
  };
}

struct GetLayerOptions {
  bool disableExposureLogging;
};

inline void to_json(json &j, const GetLayerOptions &b) {
  j = json{
      {"disable_exposure_logging", b.disableExposureLogging},
  };
}
}; // namespace statsig_cpp_core
