package com.statsig;

import static org.junit.jupiter.api.Assertions.assertNotNull;

import java.util.Arrays;
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
  void testExposureDedupeMaxKeys() {
    // Forwarded to the core StatsigOptions across the JNI boundary; an unset or
    // non-positive value falls back to the core default (100,000).
    StatsigOptions options1 = new StatsigOptions.Builder().setExposureDedupeMaxKeys(50000L).build();

    StatsigOptions options2 = new StatsigOptions.Builder().setExposureDedupeMaxKeys(0L).build();
  }

  @Test
  void testExposureDedupeMaxKeysWithOtherOptions() {
    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl("https://example.com/specs")
            .setEventLoggingMaxQueueSize(5000L)
            .setExposureDedupeMaxKeys(250000L)
            .setEnvironment("staging")
            .build();
  }

  @Test
  void testBuilderWithSpecAdapterConfigs() {
    SpecAdapterConfig httpConfig =
        new SpecAdapterConfig()
            .setAdapterType("http")
            .setSpecsUrl("https://example.com/http")
            .setInitTimeoutMs(1234L)
            .setAuthenticationMode("none");
    SpecAdapterConfig grpcConfig =
        new SpecAdapterConfig()
            .setAdapterType("grpc")
            .setSpecsUrl("https://example.com/grpc")
            .setInitTimeoutMs(5678L)
            .setCaCertPath("/path/ca")
            .setClientCertPath("/path/client")
            .setClientKeyPath("/path/key")
            .setDomainName("example.com");

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecAdapterConfigs(Arrays.asList(httpConfig, grpcConfig))
            .setSpecsUrl("https://fallback.specs")
            .build();
  }

  @Test
  void testBuilderWithGrpcWebsocketSpecAdapterConfig() {
    // gRPC websocket streaming to the Forward Proxy. Requires the native library
    // to be built with the `with_grpc` feature; building the options only maps the
    // config across the JNI boundary (the adapter is instantiated later at init).
    SpecAdapterConfig grpcConfig =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_GRPC_WEBSOCKET)
            .setSpecsUrl("http://localhost:50051")
            .setInitTimeoutMs(3000L);

    StatsigOptions options =
        new StatsigOptions.Builder().setSpecAdapterConfigs(Arrays.asList(grpcConfig)).build();
    assertNotNull(options);
  }

  @Test
  void testBuilderWithGrpcWebsocketMtlsSpecAdapterConfig() {
    // mTLS variant: exercises every TLS-related field on the config.
    SpecAdapterConfig grpcConfig =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_GRPC_WEBSOCKET)
            .setSpecsUrl("https://proxy.example.com:443")
            .setInitTimeoutMs(5000L)
            .setAuthenticationMode(AuthenticationMode.MTLS)
            .setCaCertPath("/certs/ca.pem")
            .setClientCertPath("/certs/client.pem")
            .setClientKeyPath("/certs/client.key")
            .setDomainName("proxy.example.com");

    StatsigOptions options =
        new StatsigOptions.Builder().setSpecAdapterConfigs(Arrays.asList(grpcConfig)).build();
    assertNotNull(options);
  }

  @Test
  void testBuilderWithGrpcAndHttpFallbackSpecAdapterConfigs() {
    // Ordered adapter list: gRPC websocket first, HTTP as fallback.
    SpecAdapterConfig grpcConfig =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_GRPC_WEBSOCKET)
            .setSpecsUrl("http://localhost:50051")
            .setInitTimeoutMs(3000L);
    SpecAdapterConfig httpConfig =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_HTTP)
            .setSpecsUrl("https://example.com/specs")
            .setInitTimeoutMs(3000L);

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecAdapterConfigs(Arrays.asList(grpcConfig, httpConfig))
            .setSpecsUrl("https://fallback.specs")
            .build();
    assertNotNull(options);
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
