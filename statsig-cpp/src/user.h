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
  std::string userID;
  std::unordered_map<std::string, std::string> customIDs;
  std::string email;
  std::string ip;
  std::string userAgent;
  std::string country;
  std::string locale;
  std::string appVersion;
  std::unordered_map<std::string, allowed_type> custom;
  std::unordered_map<std::string, allowed_type> privateAttribute;

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
  j.at("userID").get_to(u.userID);
  j.at("customIDs").get_to(u.customIDs);
  j.at("email").get_to(u.email);
  j.at("ip").get_to(u.ip);
  j.at("userAgent").get_to(u.userAgent);
  j.at("country").get_to(u.country);
  j.at("locale").get_to(u.locale);
  j.at("appVersion").get_to(u.appVersion);
  if (j.contains("custom") && j["custom"].is_object()) {
    u.custom.clear();
    for (auto it = j["custom"].begin(); it != j["custom"].end(); ++it) {
      try {
        allowed_primitive val;
        from_json(it.value(), val); // use your from_json for allowed_primitive
        u.custom[it.key()] = val;
      } catch (...) {
      }
    }
  }
  if (j.contains("privateAttributes") && j["privateAttributes"].is_object()) {
    u.custom.clear();
    for (auto it = j["privateAttributes"].begin();
         it != j["privateAttributes"].end(); ++it) {
      try {
        allowed_primitive val;
        from_json(it.value(), val); // use your from_json for allowed_primitive
        u.custom[it.key()] = val;
      } catch (...) {
      }
    }
  }
}

inline void to_json(json &j, const UserBuilder &u) {
  json custom_json = json{};
  to_json(custom_json, u.custom);
  json private_att_json = json{};
  to_json(private_att_json, u.privateAttribute);
  j = json{{"userID", u.userID},       {"customIDs", u.customIDs},
           {"email", u.email},         {"ip", u.ip},
           {"userAgent", u.userAgent}, {"country", u.country},
           {"locale", u.locale},       {"appVersion", u.appVersion},
           {"custom", custom_json},    {"privateAttributes", private_att_json}};
}
} // namespace statsig_cpp_core
