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

  protected StatsigUser(StatsigUserBuilder builder) {
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
  public static class Builder extends StatsigUserBuilder {
    public Builder() {
      super();
    }
  }
}
