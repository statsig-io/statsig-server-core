#pragma once

#include <iostream>
#include <nlohmann/json.hpp>
#include <string>
#include <unordered_map>
#include <variant>
using json = nlohmann::json;

template <typename T>
std::optional<T> get_optional(const json &j, const std::string &key) {
  if (j.contains(key) && !j[key].is_null()) {
    return j[key].get<T>();
  }
  return std::nullopt;
}
// Function order matters

namespace statsig_cpp_core {
struct EvaluationDetails {
  std::optional<uint64_t> lcut;
  std::optional<uint64_t> receivedAt;
  std::string reason;
};

inline void from_json(const json &j, EvaluationDetails &d) {
  d.lcut = get_optional<uint64_t>(j, "lcut");
  d.receivedAt = get_optional<uint64_t>(j, "received_at");
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
  ~FeatureGate() {
    // Nothing to manually free here because all members are RAII-safe
    // Just for demonstration
    name.clear();
    rule_id.clear();
    id_type.clear();
    // details destructor will be called automatically
  }
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
  std::optional<std::string> group_name;
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
  c.group_name = get_optional<std::string>(j, "group_name");
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

using allowed_primitive = std::variant<std::string, int64_t, double, bool>;
using allowed_type =
    std::variant<allowed_primitive, std::vector<allowed_primitive>>;

inline void from_json(const json &j, allowed_primitive &p) {
  if (j.is_string()) {
    p = j.get<std::string>();
  } else if (j.is_number_integer()) {
    p = j.get<int64_t>();
  } else if (j.is_number_float()) {
    p = j.get<double>();
  } else if (j.is_boolean()) {
    p = j.get<bool>();
  } else {
    throw std::runtime_error("Invalid type for allowed_primitive");
  }
}

inline void to_json(nlohmann::json &j, const allowed_primitive &v) {
  if (const int64_t *maybeInt = std::get_if<int64_t>(&v)) {
    j = *maybeInt;
  } else if (const double *maybeDouble = std::get_if<double>(&v)) {
    j = *maybeDouble;
  } else if (const std::string *maybeString = std::get_if<std::string>(&v)) {
    j = *maybeString;
  } else if (const bool *maybeBool = std::get_if<bool>(&v)) {
    j = *maybeBool;
  }
}

inline void to_json(json &j, const allowed_type &at) {
  std::visit(
      [&j](auto &&arg) {
        using T = std::decay_t<decltype(arg)>;
        if constexpr (std::is_same_v<T, allowed_primitive>) {
          to_json(j, arg);
        } else if constexpr (std::is_same_v<T,
                                            std::vector<allowed_primitive>>) {
          j = json::array();
          for (const auto &elem : arg) {
            json je;
            to_json(je, elem);
            j.push_back(je);
          }
        }
      },
      at);
}

inline void from_json(const json &j, allowed_type &at) {
  if (j.is_array()) {
    std::vector<allowed_primitive> vec;
    for (const auto &item : j) {
      allowed_primitive ap;
      from_json(item, ap);
      vec.push_back(ap);
    }
    at = vec;
  } else {
    allowed_primitive ap;
    from_json(j, ap);
    at = ap;
  }
}

inline void to_json(nlohmann::json &j,
                    const std::unordered_map<std::string, allowed_type> &m) {
  j = nlohmann::json::object();
  for (const auto &[key, value] : m) {
    json vj = json{};
    to_json(vj, value);
    j[key] = vj;
  }
}
inline void from_json(const json &j,
                      std::unordered_map<std::string, allowed_type> &m) {
  for (auto it = j.begin(); it != j.end(); ++it) {
    allowed_type p;
    from_json(it.value(), p);
    m[it.key()] = std::move(p);
  }
}
} // namespace statsig_cpp_core
