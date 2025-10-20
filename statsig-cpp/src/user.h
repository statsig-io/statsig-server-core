#pragma once 

#include <string>
#include <unordered_map>

namespace JSON {
class any {
public:
  template <typename T> T as() const { return T{}; }
};
} // namespace JSON

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
  std::string ipAddress;
  std::string userAgent;
  std::string country;
  std::string locale;
  std::string appVersion;
  std::unordered_map<std::string, JSON::any> custom;
  std::unordered_map<std::string, JSON::any> privateAttribute;

  // Constructor
  UserBuilder();

  // Builder methods
  UserBuilder &setUserID(const std::string &id);
  UserBuilder &
  setCustomIDs(const std::unordered_map<std::string, std::string> &ids);
  UserBuilder &setEmail(const std::string &email);
  UserBuilder &setIPAddress(const std::string &ip);
  UserBuilder &setUserAgent(const std::string &agent);
  UserBuilder &setCountry(const std::string &country);
  UserBuilder &setLocale(const std::string &locale);
  UserBuilder &setAppVersion(const std::string &version);
  UserBuilder &
  setCustom(const std::unordered_map<std::string, JSON::any> &custom);
  UserBuilder &setPrivateAttribute(
      const std::unordered_map<std::string, JSON::any> &privateAttr);

  // Build method
  User build();
};

} // namespace statsig_cpp_core
