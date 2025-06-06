package com.statsig;

import java.util.Map;

public class StatsigUserBuilder {
  public String userID;
  public Map<String, String> customIDs;
  public String email;
  public String ip;
  public String locale;
  public String appVersion;
  public String userAgent;
  public String country;
  public Map<String, String> privateAttributes;
  public Map<String, Object> custom;

  public StatsigUserBuilder setUserID(String userID) {
    this.userID = userID;
    return this;
  }

  public StatsigUserBuilder setCustomIDs(Map<String, String> customIDs) {
    this.customIDs = customIDs;
    return this;
  }

  public StatsigUserBuilder setEmail(String email) {
    this.email = email;
    return this;
  }

  public StatsigUserBuilder setIp(String ip) {
    this.ip = ip;
    return this;
  }

  public StatsigUserBuilder setLocale(String locale) {
    this.locale = locale;
    return this;
  }

  public StatsigUserBuilder setAppVersion(String appVersion) {
    this.appVersion = appVersion;
    return this;
  }

  public StatsigUserBuilder setUserAgent(String userAgent) {
    this.userAgent = userAgent;
    return this;
  }

  public StatsigUserBuilder setCountry(String country) {
    this.country = country;
    return this;
  }

  public StatsigUserBuilder setPrivateAttributes(Map<String, String> privateAttributes) {
    this.privateAttributes = privateAttributes;
    return this;
  }

  public StatsigUserBuilder setCustom(Map<String, Object> custom) {
    this.custom = custom;
    return this;
  }

  public StatsigUser build() {
    return new StatsigUser(this);
  }
}
