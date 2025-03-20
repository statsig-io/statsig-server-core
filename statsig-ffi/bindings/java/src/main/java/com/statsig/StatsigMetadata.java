package com.statsig;

import com.statsig.internal.GsonUtil;

import java.util.Map;

public class StatsigMetadata {
    public static String getSerializedCopy() {
        Map<String, String> metadata = Map.of(
            "os", System.getProperty("os.name"),
            "arch", System.getProperty("os.arch"),
            "languageVersion", System.getProperty("java.version")
        );

        return GsonUtil.getGson().toJson(metadata);
    }
}