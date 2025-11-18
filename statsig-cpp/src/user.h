#pragma once

#include "types.h"
#include <nlohmann/json.hpp>
#include <string>
#include <unordered_map>
using json = nlohmann::json;

namespace statsig_cpp_core {
struct User {
  uint64_t ref;
  User(const uint64_t ref);
  ~User();
};

struct UserBuilder {
  std::optional<std::string> userID;
  std::optional<std::unordered_map<std::string, std::string>> customIDs;
  std::optional<std::string> email;
  std::optional<std::string> ip;
  std::optional<std::string> userAgent;
  std::optional<std::string> country;
  std::optional<std::string> locale;
  std::optional<std::string> appVersion;
  std::optional<std::unordered_map<std::string, allowed_type>> custom;
  std::optional<std::unordered_map<std::string, allowed_type>> privateAttribute;

  // Constructor
  UserBuilder();

  // Builder methods
  UserBuilder &setUserID(const std::string &id);
  UserBuilder &
  setCustomIDs(const std::unordered_map<std::string, std::string> &ids);
  UserBuilder &setEmail(const std::string &email);
  UserBuilder &setIp(const std::string &ip);
  UserBuilder &setUserAgent(const std::string &agent);
  UserBuilder &setCountry(const std::string &country);
  UserBuilder &setLocale(const std::string &locale);
  UserBuilder &setAppVersion(const std::string &version);
  UserBuilder &
  setCustom(const std::unordered_map<std::string, allowed_type> &custom);
  UserBuilder &setPrivateAttribute(
      const std::unordered_map<std::string, allowed_type> &privateAttr);

  // Build method
  User build();
};

inline void from_json(const json &j, UserBuilder &u) {
  u.userID = get_optional<std::string>(j, "userID");
  u.customIDs = get_optional<std::unordered_map<std::string, std::string>>(
      j, "customIDs");
  u.email = get_optional<std::string>(j, "email");
  u.ip = get_optional<std::string>(j, "ip");
  u.userAgent = get_optional<std::string>(j, "userAgent");
  u.country = get_optional<std::string>(j, "country");
  u.locale = get_optional<std::string>(j, "locale");
  u.appVersion = get_optional<std::string>(j, "appVersion");
  if (j.contains("custom") && j["custom"].is_object()) {
    u.custom = std::unordered_map<std::string, allowed_type>{};
    for (auto it = j["custom"].begin(); it != j["custom"].end(); ++it) {
      try {
        allowed_type val;
        from_json(it.value(), val); 
        u.custom.value()[it.key()] = val;
      } catch (const std::exception &e) {
        std::cerr << "[Statsig::User]" << "Failed to parse custom: " << it.key()
                  << " | error: " << e.what() << std::endl;
      }
    }
  }
  if (j.contains("privateAttributes") && j["privateAttributes"].is_object()) {
    u.privateAttribute = std::unordered_map<std::string, allowed_type>{};
    for (auto it = j["privateAttributes"].begin();
         it != j["privateAttributes"].end(); ++it) {
      try {
        allowed_type val;
        from_json(it.value(), val);
        u.privateAttribute.value()[it.key()] = val;
      } catch (const std::exception &e) {
        std::cerr << "[Statsig::User]" << "Failed to parse private attribute: " << it.key()
                  << " | error: " << e.what() << std::endl;
      }
    }
  }
}

inline void to_json(json &j, const UserBuilder &u) {
  json custom_json = json{};
  if (u.custom.has_value()) {
    to_json(custom_json, u.custom.value());
  }
  json private_att_json = json{};
  if (u.privateAttribute.has_value()) {
    to_json(private_att_json, u.privateAttribute.value());
  }
  j = json{{"userID", u.userID},       {"customIDs", u.customIDs},
           {"email", u.email},         {"ip", u.ip},
           {"userAgent", u.userAgent}, {"country", u.country},
           {"locale", u.locale},       {"appVersion", u.appVersion},
           {"custom", custom_json},    {"privateAttributes", private_att_json}};
}
} // namespace statsig_cpp_core
