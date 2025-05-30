package com.statsig;

import com.statsig.internal.JacksonUtil;
import java.util.Map;

public class StatsigUser {
  private String userID;
  private Map<String, String> customIDs;
  private String email;
  private String ip;
  private String locale;
  private String appVersion;
  private String userAgent;
  private String country;
  private Map<String, String> privateAttributes;
  private Map<String, Object> custom;

  private volatile long ref;

  /** Default constructor for Jackson deserialization. */
  public StatsigUser() {}

  private static class CleaningAction implements Runnable {
    private final long ref;

    CleaningAction(long ref) {
      this.ref = ref;
    }

    @Override
    public void run() {
      StatsigJNI.statsigUserRelease(ref);
    }
  }

  protected StatsigUser(Builder builder) {
    this.userID = builder.userID;
    this.customIDs = builder.customIDs;
    this.email = builder.email;
    this.ip = builder.ip;
    this.locale = builder.locale;
    this.appVersion = builder.appVersion;
    this.userAgent = builder.userAgent;
    this.country = builder.country;
    this.privateAttributes = builder.privateAttributes;
    this.custom = builder.custom;

    initializeRef();
    ResourceCleaner.register(this, new CleaningAction(this.ref));
  }

  private void initializeRef() {
    String customIdsJson = JacksonUtil.toJson(customIDs);
    String customJson = JacksonUtil.toJson(custom);
    String privateAttributesJson = JacksonUtil.toJson(privateAttributes);

    // Pass all arguments to the JNI binding
    this.ref =
        StatsigJNI.statsigUserCreate(
            userID,
            customIdsJson,
            email,
            ip,
            userAgent,
            country,
            locale,
            appVersion,
            customJson,
            privateAttributesJson);
  }

  // Expose a way for users to force release StatsigUser
  public void close() {
    if (ref != 0) {
      StatsigJNI.statsigUserRelease(ref);
      ref = 0;
    }
  }

  public String getUserID() {
    return userID;
  }

  public Map<String, String> getCustomIDs() {
    return customIDs;
  }

  public String getEmail() {
    return email;
  }

  public String getIp() {
    return ip;
  }

  public String getLocale() {
    return locale;
  }

  public String getAppVersion() {
    return appVersion;
  }

  public String getUserAgent() {
    return userAgent;
  }

  public String getCountry() {
    return country;
  }

  public Map<String, String> getPrivateAttributes() {
    return privateAttributes;
  }

  public Map<String, Object> getCustom() {
    return custom;
  }

  long getRef() {
    return ref;
  }

  // Builder Class
  public static class Builder {
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

    public Builder setUserID(String userID) {
      this.userID = userID;
      return this;
    }

    public Builder setCustomIDs(Map<String, String> customIDs) {
      this.customIDs = customIDs;
      return this;
    }

    public Builder setEmail(String email) {
      this.email = email;
      return this;
    }

    public Builder setIp(String ip) {
      this.ip = ip;
      return this;
    }

    public Builder setLocale(String locale) {
      this.locale = locale;
      return this;
    }

    public Builder setAppVersion(String appVersion) {
      this.appVersion = appVersion;
      return this;
    }

    public Builder setUserAgent(String userAgent) {
      this.userAgent = userAgent;
      return this;
    }

    public Builder setCountry(String country) {
      this.country = country;
      return this;
    }

    public Builder setPrivateAttributes(Map<String, String> privateAttributes) {
      this.privateAttributes = privateAttributes;
      return this;
    }

    public Builder setCustom(Map<String, Object> custom) {
      this.custom = custom;
      return this;
    }

    public StatsigUser build() {
      return new StatsigUser(this);
    }
  }
}
