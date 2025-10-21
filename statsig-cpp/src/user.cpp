#include "user.h"
#include "libstatsig_ffi.h"
#include <cstring>
#include <string>
#include <unordered_map>

namespace statsig_cpp_core {

// User implementation
User::User(const uint64_t ref) : ref(ref) {}

User::~User() {
  if (ref != 0) {
    statsig_user_release(ref);
  }
}

// UserBuilder implementation
UserBuilder::UserBuilder() {
  // Initialize with default values
  userID = "";
  customIDs.clear();
  email = "";
  ipAddress = "";
  userAgent = "";
  country = "";
  locale = "";
  appVersion = "";
  custom.clear();
  privateAttribute.clear();
}

UserBuilder &UserBuilder::setUserID(const std::string &id) {
  userID = id;
  return *this;
}

UserBuilder &UserBuilder::setCustomIDs(
    const std::unordered_map<std::string, std::string> &ids) {
  customIDs = ids;
  return *this;
}

UserBuilder &UserBuilder::setEmail(const std::string &email) {
  this->email = email;
  return *this;
}

UserBuilder &UserBuilder::setIPAddress(const std::string &ip) {
  ipAddress = ip;
  return *this;
}

UserBuilder &UserBuilder::setUserAgent(const std::string &agent) {
  userAgent = agent;
  return *this;
}

UserBuilder &UserBuilder::setCountry(const std::string &country) {
  this->country = country;
  return *this;
}

UserBuilder &UserBuilder::setLocale(const std::string &locale) {
  this->locale = locale;
  return *this;
}

UserBuilder &UserBuilder::setAppVersion(const std::string &version) {
  appVersion = version;
  return *this;
}

UserBuilder &UserBuilder::setCustom(
    const std::unordered_map<std::string, JSON::any> &custom) {
  this->custom = custom;
  return *this;
}

UserBuilder &UserBuilder::setPrivateAttribute(
    const std::unordered_map<std::string, JSON::any> &privateAttr) {
  privateAttribute = privateAttr;
  return *this;
}

User UserBuilder::build() {
  // Convert customIDs map to JSON string
  std::string customIDsJson = "{}";
  if (!customIDs.empty()) {
    customIDsJson = "{";
    bool first = true;
    for (const auto &pair : customIDs) {
      if (!first)
        customIDsJson += ",";
      customIDsJson += "\"" + pair.first + "\":\"" + pair.second + "\"";
      first = false;
    }
    customIDsJson += "}";
  }

  // Convert custom map to JSON string
  std::string customJson = "{}";
  if (!custom.empty()) {
    customJson = "{";
    bool first = true;
    for (const auto &pair : custom) {
      if (!first)
        customJson += ",";
      customJson +=
          "\"" + pair.first + "\":\"" + pair.second.as<std::string>() + "\"";
      first = false;
    }
    customJson += "}";
  }

  // Convert private attributes map to JSON string
  std::string privateAttributesJson = "{}";
  if (!privateAttribute.empty()) {
    privateAttributesJson = "{";
    bool first = true;
    for (const auto &pair : privateAttribute) {
      if (!first)
        privateAttributesJson += ",";
      privateAttributesJson +=
          "\"" + pair.first + "\":\"" + pair.second.as<std::string>() + "\"";
      first = false;
    }
    privateAttributesJson += "}";
  }

  // Create user using FFI function
  uint64_t userRef = statsig_user_create(
      userID.empty() ? nullptr : userID.c_str(), customIDsJson.c_str(),
      email.empty() ? nullptr : email.c_str(),
      ipAddress.empty() ? nullptr : ipAddress.c_str(),
      userAgent.empty() ? nullptr : userAgent.c_str(),
      country.empty() ? nullptr : country.c_str(),
      locale.empty() ? nullptr : locale.c_str(),
      appVersion.empty() ? nullptr : appVersion.c_str(), customJson.c_str(),
      privateAttributesJson.c_str());

  return User(userRef);
}

} // namespace statsig_cpp_core