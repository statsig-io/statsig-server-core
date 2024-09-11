package com.statsig;

import com.google.gson.Gson;

import java.util.Map;

public class StatsigUser implements AutoCloseable {
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

    // Just to make test easier
    public StatsigUser(String userId, String email) {
        this(userId, null, email, null, null, null, null, null, null, null);
    }

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

    @Override
    public synchronized void close() {
        if (ref != null) {
            StatsigJNI.statsigUserRelease(this.ref);
            this.ref = null;
        }
    }

    String getRef() {
        return ref;
    }
}
