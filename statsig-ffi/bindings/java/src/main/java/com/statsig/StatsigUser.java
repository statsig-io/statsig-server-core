package com.statsig;

import com.google.gson.Gson;

import java.util.Map;

public class StatsigUser implements AutoCloseable {
    private long ref;

    // Just to make test easier
    public StatsigUser(String userId, String email) {
        this.ref = StatsigJNI.statsigUserCreate(userId, null, email, null,
                null, null, null,
                null, null, null);
    }

    public StatsigUser(
            String userId,
            Map<String, String> customIds,
            String email,
            String ip,
            String userAgent,
            String country,
            String locale,
            String appVersion,
            Map<String, String> custom,
            Map<String, String> privateAttributes
    ) {
        Gson gson = new Gson();

        String customIdsJson = customIds != null ? gson.toJson(customIds) : null;
        String customJson = custom != null ? gson.toJson(custom) : null;
        String privateAttributesJson = privateAttributes != null ? gson.toJson(privateAttributes) : null;


        // Pass all arguments to the JNI binding
        this.ref = StatsigJNI.statsigUserCreate(
                userId,
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
    public void close() {

        if (ref != 0) {
StatsigJNI.statsigUserRelease(this.ref);
            this.ref = 0;
        }
    }

    public long getRef() {
        return ref;
    }
}
