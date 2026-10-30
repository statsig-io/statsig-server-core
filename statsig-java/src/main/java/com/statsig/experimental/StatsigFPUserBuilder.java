package com.statsig.experimental;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONObject;
import com.statsig.ExperimentalUserEnrichmentBridge;
import com.statsig.StatsigUser;
import java.util.Map;
import java.util.concurrent.CompletableFuture;

/**
 * <b>Experimental.</b> Builds a {@link StatsigUser} whose ID-list memberships are resolved through
 * a Statsig forward proxy ({@code POST /v1/user_enrichment}) before evaluation. This API is
 * unstable and may change or be removed without notice.
 */
public class StatsigFPUserBuilder {
  private final ForwardProxyUserEnrichmentProvider provider;
  private String userID;
  private Map<String, String> customIDs;
  private String email;
  private String ip;
  private String userAgent;
  private String country;
  private String locale;
  private String appVersion;
  private Map<String, Object> custom;
  private Map<String, String> privateAttributes;

  public StatsigFPUserBuilder(ForwardProxyUserEnrichmentProvider provider) {
    this.provider = java.util.Objects.requireNonNull(provider, "provider");
  }

  public StatsigFPUserBuilder setUserID(String userID) {
    this.userID = userID;
    return this;
  }

  public StatsigFPUserBuilder setCustomIDs(Map<String, String> customIDs) {
    this.customIDs = customIDs;
    return this;
  }

  public StatsigFPUserBuilder setEmail(String email) {
    this.email = email;
    return this;
  }

  public StatsigFPUserBuilder setIp(String ip) {
    this.ip = ip;
    return this;
  }

  public StatsigFPUserBuilder setUserAgent(String userAgent) {
    this.userAgent = userAgent;
    return this;
  }

  public StatsigFPUserBuilder setCountry(String country) {
    this.country = country;
    return this;
  }

  public StatsigFPUserBuilder setLocale(String locale) {
    this.locale = locale;
    return this;
  }

  public StatsigFPUserBuilder setAppVersion(String appVersion) {
    this.appVersion = appVersion;
    return this;
  }

  public StatsigFPUserBuilder setCustom(Map<String, Object> custom) {
    this.custom = custom;
    return this;
  }

  public StatsigFPUserBuilder setPrivateAttributes(Map<String, String> privateAttributes) {
    this.privateAttributes = privateAttributes;
    return this;
  }

  public CompletableFuture<StatsigUser> build() {
    StatsigUser.Builder userData = snapshotUserData();
    return ExperimentalUserEnrichmentBridge.buildAsync(
        provider.getRef(), JSON.toJSONString(userData), userData);
  }

  public StatsigUser buildBlocking() {
    return buildBlocking(snapshotUserData());
  }

  private StatsigUser.Builder snapshotUserData() {
    // Use the same field values for native construction and the returned Java user,
    // even if this builder is changed while asynchronous enrichment is in flight.
    return new StatsigUser.Builder()
        .setUserID(userID)
        .setCustomIDs(customIDs)
        .setEmail(email)
        .setIp(ip)
        .setUserAgent(userAgent)
        .setCountry(country)
        .setLocale(locale)
        .setAppVersion(appVersion)
        .setCustom(custom)
        .setPrivateAttributes(privateAttributes);
  }

  private StatsigUser buildBlocking(StatsigUser.Builder userData) {
    String resultJson =
        ExperimentalUserEnrichmentBridge.buildBlocking(
            provider.getRef(), JSON.toJSONString(userData));
    JSONObject result = JSON.parseObject(resultJson);
    if (result.containsKey("error")) {
      throw new EnrichmentException(result.getString("error"));
    }
    return ExperimentalUserEnrichmentBridge.createUser(result.getLongValue("userRef"), userData);
  }
}
