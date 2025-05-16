package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

public class ObservabilityClientTest {

  // Mock OB
  public static class MockObservabilityClient implements ObservabilityClient {
    public List<String> calledMethods = new ArrayList<>();
    public List<MetricEvent> metrics = new ArrayList<>();
    public boolean errorCalled = false;

    public static class MetricEvent {
      public final String type;
      public final String name;
      public final Double value;
      public final Map<String, String> tags;

      public MetricEvent(String type, String name, Double value, Map<String, String> tags) {
        this.type = type;
        this.name = name;
        this.value = value;
        this.tags = tags;
      }
    }

    @Override
    public void init() {
      calledMethods.add("init");
    }

    @Override
    public void increment(String metricName, double value, Map<String, String> tags) {
      calledMethods.add("increment");
      metrics.add(new MetricEvent("increment", metricName, value, tags));
    }

    @Override
    public void gauge(String metricName, double value, Map<String, String> tags) {
      calledMethods.add("guage");
      metrics.add(new MetricEvent("gauge", metricName, value, tags));
    }

    @Override
    public void dist(String metricName, double value, Map<String, String> tags) {
      calledMethods.add("dist");
      metrics.add(new MetricEvent("dist", metricName, value, tags));
    }

    @Override
    public void error(String tag, String message) {
      errorCalled = true;
    }
  }

  @Test
  public void testObservabilityClientUsage() throws Exception {
    MockObservabilityClient mockClient = new MockObservabilityClient();

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setObservabilityClient(mockClient)
            .setOutputLoggerLevel(OutputLogger.LogLevel.ERROR)
            .setSpecsSyncIntervalMs(1)
            .build();

    Statsig statsig = new Statsig("secret-key", options);
    statsig.initialize().get();

    StatsigUser user =
        new StatsigUser.Builder().setUserID("123").setEmail("weihao@statsig.com").build();
    statsig.checkGate(user, "test-gate");

    statsig.flushEvents().get();
    statsig.shutdown().get();

    Assertions.assertTrue(mockClient.calledMethods.contains("dist"));
    Assertions.assertTrue(mockClient.calledMethods.contains("init"));

    MockObservabilityClient.MetricEvent distMetric = null;

    for (MockObservabilityClient.MetricEvent m : mockClient.metrics) {
      if ("dist".equals(m.type) && "statsig.sdk.initialization".equals(m.name)) {
        distMetric = m;
        break;
      }
    }

    // todo working on some more tests, but the idea is similar
    assertNotNull(distMetric, "Expected dist metric to be present");
    assertEquals("true", distMetric.tags.get("success"));
  }
}
