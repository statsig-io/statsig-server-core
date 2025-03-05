package com.statsig;

import org.junit.jupiter.api.Test;

public class StatsigOptionsTest {
    @Test
    void testBuilderDefaultValues() {
        StatsigOptions options = new StatsigOptions.Builder().build();
    }

    @Test
    void testBuilderSetAllValues() {
        StatsigOptions options = new StatsigOptions.Builder()
                .setSpecsUrl("https://example.com/specs")
                .setLogEventUrl("https://example.com/log")
                .setIdListsUrl("https://example.com/idlists")
                .setSpecsSyncIntervalMs(1000L)
                .setEventLoggingFlushIntervalMs(2000L)
                .setEventLoggingMaxQueueSize(5000L)
                .setEnvironment("staging")
                .setDisableAllLogging(true)
                .setEnableIDLists(true)
                .setEnableUserAgentParsing(true)
                .setEnableCountryLookup(true)
                .setServiceName("test_service")
                .setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG)
                .build();
    }

    @Test
    void testBuilderSetNumericValues() {
        StatsigOptions options = new StatsigOptions.Builder()
                .setSpecsSyncIntervalMs(12345L)
                .setEventLoggingFlushIntervalMs(67890L)
                .setEventLoggingMaxQueueSize(111213L)
                .build();
    }

    @Test
    void testBuilderSetBooleanValues() {
        StatsigOptions options = new StatsigOptions.Builder()
                .setEnableIDLists(true)
                .setEnableUserAgentParsing(false)
                .setEnableCountryLookup(true)
                .setDisableAllLogging(false)
                .build();
    }

    @Test
    void testBuilderSetStringValues() {
        StatsigOptions options = new StatsigOptions.Builder()
                .setSpecsUrl("https://example.com/specs")
                .setLogEventUrl("")
                .setIdListsUrl(null)
                .setEnvironment("production")
                .setServiceName("statsig_service")
                .build();
    }

    @Test
    void testBuilderEmptyValues() {
        StatsigOptions options = new StatsigOptions.Builder()
                .setSpecsUrl("")
                .setLogEventUrl("")
                .setIdListsUrl("")
                .setEnvironment("")
                .setServiceName("")
                .build();
    }
}
