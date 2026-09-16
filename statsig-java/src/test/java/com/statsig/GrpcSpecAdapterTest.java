package com.statsig;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Arrays;
import org.junit.jupiter.api.Test;

/**
 * Regression coverage for the {@code with_grpc} cargo feature being enabled in the native library
 * shipped with javacore.
 *
 * <p>Without it, {@code StatsigCustomizedSpecsAdapter::create_grpc_adapter} falls through to its
 * {@code #[cfg(not(feature = "with_grpc"))]} stub, which logs an error and returns no adapter — the
 * exact failure reported against the published javacore build.
 */
public class GrpcSpecAdapterTest {

  private static final String FEATURE_DISABLED_LOG =
      "Trying to use grpc websocket adapter but with grpc feature is not enabled";

  @Test
  public void testGrpcWebsocketAdapterIsCompiledIntoNativeLibrary() throws Exception {
    OutputLoggerProviderTest.MockOutputLoggerProvider logger =
        new OutputLoggerProviderTest.MockOutputLoggerProvider();

    // Nothing is listening on this port. The adapter cannot connect, but it must still be
    // *constructed* — construction is what the with_grpc feature gates.
    SpecAdapterConfig grpcConfig =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_GRPC_WEBSOCKET)
            .setSpecsUrl("http://localhost:50051")
            .setInitTimeoutMs(1000L);

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecAdapterConfigs(Arrays.asList(grpcConfig))
            .setOutputLoggerProvider(logger)
            .setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG)
            .build();

    Statsig statsig = new Statsig("secret-key", options);
    try {
      statsig.initialize().get();
    } finally {
      statsig.shutdown().get();
    }

    // The assertion below is an absence check, so it passes vacuously if the mock provider was
    // never installed. initialize_output_logger is a process-global first-caller-wins latch, and
    // all statsig-java tests share one JVM — any earlier test that constructs a Statsig without
    // shutting it down claims the latch and leaves this test blind. Fail loudly instead.
    assertTrue(
        logger.calledMethods.contains("init"),
        "Mock output logger was never installed — an earlier test claimed the process-global"
            + " output logger and did not release it. The gRPC assertion below would be vacuous.");
    assertFalse(logger.logMessages.isEmpty(), "No log messages were captured.");

    boolean sawFeatureDisabled =
        logger.logMessages.stream().anyMatch(msg -> msg.message.contains(FEATURE_DISABLED_LOG));

    assertFalse(
        sawFeatureDisabled,
        "Native library was built without the with_grpc feature — gRPC websocket streaming to the"
            + " Forward Proxy is unavailable. Check the statsig-rust feature list in"
            + " statsig-ffi/Cargo.toml.");
  }

  @Test
  public void testUnsetInitTimeoutUsesCoreDefaultRatherThanZero() throws Exception {
    // SpecAdapterConfig.initTimeoutMs is a primitive long, so leaving it unset reads as 0 across
    // JNI. A 0 ms timeout elapses on its first poll, so the adapter could never start — even
    // against a healthy Forward Proxy. The JNI read treats 0 as unset and falls back to
    // DEFAULT_INIT_TIMEOUT_MS (3000).
    SpecAdapterConfig grpcConfig =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_GRPC_WEBSOCKET)
            .setSpecsUrl("http://localhost:50051");

    StatsigOptions options =
        new StatsigOptions.Builder().setSpecAdapterConfigs(Arrays.asList(grpcConfig)).build();

    Statsig statsig = new Statsig("secret-key", options);
    long startNanos = System.nanoTime();
    try {
      statsig.initialize().get();
    } finally {
      statsig.shutdown().get();
    }
    long elapsedMs = (System.nanoTime() - startNanos) / 1_000_000L;

    // Nothing is listening on that port, so the adapter waits out its init timeout. With the 3000
    // ms default that is ~3.3s; with the 0 ms bug it returns in ~0.4s. 1500 ms separates them
    // with wide margin on both sides.
    assertTrue(
        elapsedMs >= 1500,
        "Initialize returned after "
            + elapsedMs
            + " ms, so the gRPC adapter was given a 0 ms init timeout instead of the "
            + "DEFAULT_INIT_TIMEOUT_MS fallback. An unset initTimeoutMs must not mean 'time out "
            + "immediately'.");
  }
}
