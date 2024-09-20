package com.statsig;

import com.google.gson.Gson;

import java.util.Map;

public class StatsigUser {
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

    private volatile String ref;

    public StatsigUser(
            String userId,
            Map<String, String> customIDs,
            String email,
            String ip,
            String userAgent,
            String country,
            String locale,
            String appVersion,
            Map<String, Object> custom,
            Map<String, String> privateAttributes
    ) {
        this.userID = userId;
        this.custom = custom;
        this.email = email;
        this.ip = ip;
        this.locale = locale;
        this.appVersion = appVersion;
        this.customIDs = customIDs;
        this.privateAttributes = privateAttributes;
        this.country = country;
        this.userAgent = userAgent;

        initializeRef();
        ResourceCleaner.register(this, () -> {
            if (ref != null) {
                StatsigJNI.statsigUserRelease(ref);
                ref = null;
            }
        });
    }

    public StatsigUser(String userId) {
        this(userId, null, null, null, null, null, null, null, null, null);
    }

    public StatsigUser(Map<String, String> customIDs) {
        this(null, customIDs, null, null, null, null, null, null, null, null);
    }

    public StatsigUser(String userId, String email) {
        this(userId, null, email, null, null, null, null, null, null, null);
    }

    public String getUserID() {
        return userID;
    }

    public void setUserID(String userID) {
        this.userID = userID;
    }

    public Map<String, String> getCustomIDs() {
        return customIDs;
    }

    public void setCustomIDs(Map<String, String> customIDs) {
        this.customIDs = customIDs;
    }

    public String getEmail() {
        return email;
    }

    public void setEmail(String email) {
        this.email = email;
    }

    public String getIp() {
        return ip;
    }

    public void setIp(String ip) {
        this.ip = ip;
    }

    public String getLocale() {
        return locale;
    }

    public void setLocale(String locale) {
        this.locale = locale;
    }

    public String getAppVersion() {
        return appVersion;
    }

    public void setAppVersion(String appVersion) {
        this.appVersion = appVersion;
    }

    public String getUserAgent() {
        return userAgent;
    }

    public void setUserAgent(String userAgent) {
        this.userAgent = userAgent;
    }

    public String getCountry() {
        return country;
    }

    public void setCountry(String country) {
        this.country = country;
    }

    public Map<String, String> getPrivateAttributes() {
        return privateAttributes;
    }

    public void setPrivateAttributes(Map<String, String> privateAttributes) {
        this.privateAttributes = privateAttributes;
    }

    public Map<String, Object> getCustom() {
        return custom;
    }

    public void setCustom(Map<String, Object> custom) {
        this.custom = custom;
    }

    private void initializeRef() {
        Gson gson = new Gson();

        String customIdsJson = customIDs != null ? gson.toJson(customIDs) : null;
        String customJson = custom != null ? gson.toJson(custom) : null;
        String privateAttributesJson = privateAttributes != null ? gson.toJson(privateAttributes) : null;

        // Pass all arguments to the JNI binding
        this.ref = StatsigJNI.statsigUserCreate(
                userID,
                customIdsJson,
                email,
                ip,
                userAgent,
                country,
                locale,
                appVersion,
                customJson,
                privateAttributesJson
        );
    }

    String getRef() {
        return ref;
    }
}
