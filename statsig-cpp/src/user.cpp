#include "user.h"
#include "libstatsig_ffi.h"
#include "types.h"
#include <cstring>
#include <nlohmann/json.hpp>
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
  ip = "";
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

UserBuilder &UserBuilder::setIp(const std::string &ip) {
  this->ip = ip;
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
    const std::unordered_map<std::string, allowed_type> &custom) {
  this->custom = custom;
  return *this;
}

UserBuilder &UserBuilder::setPrivateAttribute(
    const std::unordered_map<std::string, allowed_type> &privateAttr) {
  privateAttribute = privateAttr;
  return *this;
}

User UserBuilder::build() {
  // TODO(xinli): Rethink the decision here on serialization
  json j = *this;
  uint64_t userRef = statsig_user_create_from_data(j.dump().c_str());

  return User(userRef);
}

} // namespace statsig_cpp_core