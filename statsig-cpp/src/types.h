#pragma once 

#include <nlohmann/json.hpp>
#include <string>
#include <unordered_map>

using json = nlohmann::json;

// Function order matters

namespace statsig_cpp_core {
struct EvaluationDetails {
  uint64_t lcut;
  uint64_t receivedAt;
  std::string reason;
};

inline void from_json(const json &j, EvaluationDetails &d) {
  j.at("lcut").get_to(d.lcut);
  j.at("received_at").get_to(d.receivedAt);
  j.at("reason").get_to(d.reason);
}

inline void to_json(json &j, const EvaluationDetails &d) {
  j = json{
      {"lcut", d.lcut}, {"reason", d.reason}, {"receivedAt", d.receivedAt}};
}

//  FeatureGate
struct FeatureGate {
  std::string name;
  bool value;
  std::string rule_id;
  std::string id_type;
  EvaluationDetails details;

  FeatureGate() = default;
  FeatureGate(const std::string &json_str);
};

inline void from_json(const json &j, FeatureGate &c) {
  j.at("name").get_to(c.name);
  j.at("value").get_to(c.value);
  j.at("rule_id").get_to(c.rule_id);
  j.at("id_type").get_to(c.id_type);
  j.at("details").get_to(c.details);
}

inline void to_json(json &j, const FeatureGate &c) {
  j = json{{"name", c.name},
           {"value", c.value},
           {"rule_id", c.rule_id},
           {"id_type", c.id_type},
           {"details", c.details}};
}

inline FeatureGate::FeatureGate(const std::string &json_str) {
  nlohmann::json j = nlohmann::json::parse(json_str);
  *this = j.get<FeatureGate>();
}

// DynamicConfig
struct DynamicConfig {
  std::string name;
  std::unordered_map<std::string, nlohmann::json> value;
  std::string rule_id;
  std::string id_type;
  EvaluationDetails details;

  DynamicConfig() = default;
  DynamicConfig(const std::string &json_str);
};
inline void from_json(const json &j, DynamicConfig &c) {
  j.at("name").get_to(c.name);
  j.at("value").get_to(c.value);
  j.at("rule_id").get_to(c.rule_id);
  j.at("id_type").get_to(c.id_type);
  j.at("details").get_to(c.details);
}

inline void to_json(json &j, const DynamicConfig &c) {
  j = json{{"name", c.name},
           {"value", c.value},
           {"rule_id", c.rule_id},
           {"id_type", c.id_type},
           {"details", c.details}};
}

inline DynamicConfig::DynamicConfig(const std::string &json_str) {
  nlohmann::json j = nlohmann::json::parse(json_str);
  *this = j.get<DynamicConfig>();
}

// Experiment
struct Experiment {
  std::string name;
  std::unordered_map<std::string, nlohmann::json> value;
  std::string rule_id;
  std::string id_type;
  std::string group_name;
  EvaluationDetails details;
  bool is_experiment_active = false;

  Experiment() = default;
  Experiment(const std::string &json_str);
};

inline void from_json(const json &j, Experiment &c) {
  j.at("name").get_to(c.name);
  j.at("value").get_to(c.value);
  j.at("rule_id").get_to(c.rule_id);
  j.at("id_type").get_to(c.id_type);
  j.at("details").get_to(c.details);
}

inline void to_json(json &j, const Experiment &c) {
  j = json{{"name", c.name},
           {"value", c.value},
           {"rule_id", c.rule_id},
           {"id_type", c.id_type},
           {"details", c.details}};
}

inline Experiment::Experiment(const std::string &json_str) {
  json j = json::parse(json_str);
  *this = j.get<Experiment>();
}
} // namespace statsig_cpp_core
