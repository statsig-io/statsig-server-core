package com.statsig;

import java.util.Map;

public interface ObservabilityClient {
    void init();
    void increment(String metricName, double value, Map<String, String> tags);
    void gauge(String metricName, double value, Map<String, String> tags);
    void dist(String metricName, double value, Map<String, String> tags);
    void error(String tag, String errorMessage);
}

