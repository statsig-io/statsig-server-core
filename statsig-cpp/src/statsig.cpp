#include "statsig.h"
#include "libstatsig_ffi.h"
#include <cstring>
#include <iostream>
#include <nlohmann/json.hpp>
#include <sstream>

using json = nlohmann::json;
namespace statsig_cpp_core {

// Statsig implementation
Statsig::Statsig(const std::string &sdk_key) : sdk_key_(sdk_key) {
  ref_ = statsig_create(sdk_key.c_str(), 0);
}

Statsig::Statsig(const std::string &sdk_key, const StatsigOptions &options)
    : sdk_key_(sdk_key) {
  ref_ = statsig_create(sdk_key.c_str(), options.ref);
}

Statsig::~Statsig() {
  if (ref_ != 0) {
    statsig_release(ref_);
  }
}

Statsig &Statsig::operator=(const Statsig &other) {
  if (this != &other) {
    if (ref_ != 0) {
      statsig_release(ref_);
    }
    sdk_key_ = other.sdk_key_;
    ref_ =
        0; // This would need to be handled differently in a real implementation
  }
  return *this;
}

Statsig::Statsig(Statsig &&other) noexcept
    : ref_(other.ref_), sdk_key_(std::move(other.sdk_key_)) {
  other.ref_ = 0;
}

Statsig &Statsig::operator=(Statsig &&other) noexcept {
  if (this != &other) {
    if (ref_ != 0) {
      statsig_release(ref_);
    }
    ref_ = other.ref_;
    sdk_key_ = std::move(other.sdk_key_);
    other.ref_ = 0;
  }
  return *this;
}

// Initialization methods
void Statsig::initialize(std::function<void()> callback) {
  if (callback) {
    statsig_initialize(ref_, [](void) {
      // This is a simplified callback - in a real implementation,
      // you'd need to store and call the actual callback
    });
  } else {
    statsig_initialize(ref_, nullptr);
  }
}

void Statsig::initializeWithDetails(
    std::function<void(const std::string &)> callback) {
  if (callback) {
    statsig_initialize_with_details(ref_, [](char *result) {
      // This is a simplified callback - in a real implementation,
      // you'd need to store and call the actual callback
      if (result) {
        free_string(result);
      }
    });
  } else {
    statsig_initialize_with_details(ref_, nullptr);
  }
}

std::string Statsig::initializeWithDetailsBlocking() {
  char *result = statsig_initialize_with_details_blocking(ref_);
  if (result) {
    std::string result_str(result);
    free_string(result);
    return result_str;
  }
  return "";
}

void Statsig::initializeBlocking() { statsig_initialize_blocking(ref_); }

// Shutdown methods
void Statsig::shutdown(std::function<void()> callback) {
  if (callback) {
    statsig_shutdown(ref_, [](void) {
      // This is a simplified callback - in a real implementation,
      // you'd need to store and call the actual callback
    });
  } else {
    statsig_shutdown(ref_, nullptr);
  }
}

void Statsig::shutdownBlocking() { statsig_shutdown_blocking(ref_); }

// Event logging
void Statsig::flushEvents(std::function<void()> callback) {
  if (callback) {
    statsig_flush_events(ref_, [](void) {
      // This is a simplified callback - in a real implementation,
      // you'd need to store and call the actual callback
    });
  } else {
    statsig_flush_events(ref_, nullptr);
  }
}

void Statsig::flushEventsBlocking() { statsig_flush_events_blocking(ref_); }

void Statsig::logEvent(
    const User &user, const std::string &event_name,
    const std::unordered_map<std::string, std::string> &event_value,
    const std::string &metadata) {
  // Create JSON for the event
  std::ostringstream json_stream;
  json_stream << "{";
  json_stream << "\"eventName\":\"" << event_name << "\"";

  if (!event_value.empty()) {
    json_stream << ",\"value\":{";
    bool first = true;
    for (const auto &pair : event_value) {
      if (!first)
        json_stream << ",";
      json_stream << "\"" << pair.first << "\":\"" << pair.second << "\"";
      first = false;
    }

    json_stream << "}";
  }

  if (!metadata.empty()) {
    json_stream << ",\"metadata\":\"" << metadata << "\"";
  }

  json_stream << "}";

  statsig_log_event(ref_, user.ref, json_stream.str().c_str());
}

// Feature Gates
bool Statsig::checkGate(const User &user, const std::string &gate_name,
                        const std::optional<CheckGateOptions> &options) {
  std::string serialized_options;
  if (options) {
    json options_json = *options;
    serialized_options = options_json.dump();
  } else {
    serialized_options = "{}";
  }

  return statsig_check_gate(ref_, user.ref, gate_name.c_str(),
                            serialized_options.c_str());
}

FeatureGate
Statsig::getFeatureGate(const User &user, const std::string &gate_name,
                        const std::optional<CheckGateOptions> &options) {
  std::string serialized_options;
  if (options) {
    json options_json = *options;
    serialized_options = options_json.dump();
  } else {
    serialized_options = "{}";
  }
  char *result = statsig_get_feature_gate(ref_, user.ref, gate_name.c_str(),
                                          serialized_options.c_str());
  if (result) {
    std::string result_str(result);
    return FeatureGate(result_str);
  }
  return FeatureGate();
}

Experiment
Statsig::getExperiment(const User &user, const std::string &experiment_name,
                       const std::optional<GetExperimentOptions> &options) {
  std::string serialized_options;
  if (options) {
    json options_json = *options;
    serialized_options = options_json.dump();
  } else {
    serialized_options = "{}";
  }
  char *result = statsig_get_experiment(ref_, user.ref, experiment_name.c_str(),
                                        serialized_options.c_str());
  if (result) {
    std::string result_str(result);
    return Experiment(result_str);
  }
  return Experiment();
}

DynamicConfig
Statsig::getDynamicConfig(const User &user, const std::string &config_name,
                   const std::optional<GetDynamicConfigOptions> &options) {
  std::string serialized_options;
  if (options) {
    json options_json = *options;
    serialized_options = options_json.dump();
  } else {
    serialized_options = "{}";
  }
  char *result = statsig_get_dynamic_config(ref_, user.ref, config_name.c_str(),
                                            serialized_options.c_str());
  if (result) {
    std::string result_str(result);
    return DynamicConfig(result_str);
  }
  return DynamicConfig();
}

Layer Statsig::getLayer(const User &user, const std::string &layer_name,
                        const std::optional<GetLayerOptions> &options) {
  std::string serialized_options;
  if (options) {
    json options_json = *options;
    serialized_options = options_json.dump();
  } else {
    serialized_options = "{}";
  }
  char *result = statsig_get_layer(ref_, user.ref, layer_name.c_str(),
                                   serialized_options.c_str());

  if (result) {
    std::cout << "Got layer result: " << result << std::endl;
    std::string result_str(result);
    return Layer(ref_, result_str);
  }
  return Layer();
}

} // namespace statsig_cpp_core