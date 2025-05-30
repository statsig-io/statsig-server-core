package com.statsig;

import org.junit.jupiter.api.Test;

public class StatsigOptionsTest {
  @Test
  void testBuilderDefaultValues() {
    StatsigOptions options = new StatsigOptions.Builder().build();
  }

  @Test
  void testBuilderSetAllValues() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setDisableAllLogging(true)
            .setSpecsUrl("https://example.com/specs")
            .setLogEventUrl("https://example.com/log")
            .setIdListsUrl("https://example.com/idlists")
            .setSpecsSyncIntervalMs(1000L)
            .setEventLoggingFlushIntervalMs(2000L)
            .setEventLoggingMaxQueueSize(5000L)
            .setEnvironment("staging")
            .setEnableIDLists(true)
            .setWaitForUserAgentInit(true)
            .setWaitForCountryLookupInit(true)
            .setInitTimeoutMs(1000)
            .setServiceName("test_service")
            .setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG)
            .setIdListsSyncIntervalMs(3000L)
            .setDisableNetwork(true)
            .build();
  }

  @Test
  void testBuilderSetNumericValues() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setIdListsSyncIntervalMs(54321L)
            .setSpecsSyncIntervalMs(12345L)
            .setEventLoggingFlushIntervalMs(67890L)
            .setEventLoggingMaxQueueSize(111213L)
            .build();
  }

  @Test
  void testIdListsSyncIntervalMs() {
    StatsigOptions options = new StatsigOptions.Builder().setIdListsSyncIntervalMs(5000L).build();
  }

  @Test
  void testBuilderSetBooleanValues() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setEnableIDLists(true)
            .setWaitForUserAgentInit(false)
            .setWaitForCountryLookupInit(true)
            .setDisableAllLogging(false)
            .build();
  }

  @Test
  void testBuilderSetStringValues() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl("https://example.com/specs")
            .setLogEventUrl("")
            .setDisableNetwork(true)
            .setIdListsUrl(null)
            .setEnvironment("production")
            .setServiceName("statsig_service")
            .build();
  }

  @Test
  void testBuilderEmptyValues() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl("")
            .setLogEventUrl("")
            .setIdListsUrl("")
            .setEnvironment("")
            .setServiceName("")
            .build();
  }

  @Test
  void testInitTimeoutMs() {
    StatsigOptions options1 = new StatsigOptions.Builder().setInitTimeoutMs(5000L).build();

    StatsigOptions options2 = new StatsigOptions.Builder().setInitTimeoutMs(0L).build();

    StatsigOptions options3 = new StatsigOptions.Builder().setInitTimeoutMs(-1000L).build();
  }

  @Test
  void testInitTimeoutMsWithOtherOptions() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl("https://example.com/specs")
            .setLogEventUrl("https://example.com/log")
            .setInitTimeoutMs(4000L)
            .setSpecsSyncIntervalMs(1000L)
            .setEventLoggingFlushIntervalMs(2000L)
            .setEnvironment("staging")
            .build();
  }

  @Test
  void testInitTimeoutMsInAllValuesBuilder() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl("https://example.com/specs")
            .setLogEventUrl("https://example.com/log")
            .setIdListsUrl("https://example.com/idlists")
            .setIdListsSyncIntervalMs(3000L)
            .setSpecsSyncIntervalMs(1000L)
            .setEventLoggingFlushIntervalMs(2000L)
            .setEventLoggingMaxQueueSize(5000L)
            .setEnvironment("staging")
            .setDisableAllLogging(true)
            .setEnableIDLists(true)
            .setWaitForUserAgentInit(true)
            .setWaitForCountryLookupInit(true)
            .setInitTimeoutMs(6000L)
            .setServiceName("test_service")
            .setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG)
            .build();
  }

  @Test
  public void testMemoryUsage() {
    Runtime runtime = Runtime.getRuntime();

    long totalMemory = runtime.totalMemory(); // Current heap allocated
    long freeMemory = runtime.freeMemory(); // Free heap in allocated memory
    long usedMemoryPrev = totalMemory - freeMemory; // Used memory
    for (int i = 0; i < 1000; i++) {
      StatsigOptions opts = new StatsigOptions.Builder().build();
    }
    System.gc();
    long totalMemoryAfter = runtime.totalMemory(); // Current heap allocated
    long freeMemoryAfter = runtime.freeMemory(); // Free heap in allocated memory
    long usedMemoryAfter = totalMemoryAfter - freeMemoryAfter; // Used memory
    assert ((usedMemoryAfter - usedMemoryPrev) < 10); // Assert no memory leak
  }
}
