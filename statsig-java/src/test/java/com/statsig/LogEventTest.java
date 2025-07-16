package com.statsig;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.concurrent.ExecutionException;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;

public class LogEventTest {

  @Test
  public void testAllAPIs() throws InterruptedException, ExecutionException {
    TestLogger logger = new TestLogger();
    StatsigOptions options = new StatsigOptions.Builder().setOutputLoggerProvider(logger).build();
    Statsig statsig = new Statsig("secret-key", options);
    statsig.initialize().get();

    StatsigUser user = new StatsigUser.Builder().setUserID("userID").build();
    HashMap<String, String> metadata = new HashMap<String, String>();
    metadata.put("key1", "value");

    statsig.logEvent(user, "custom_event");
    statsig.logEvent(user, "custom_event", 12.2);
    statsig.logEvent(user, "custom_event", 12.2, metadata);
    statsig.logEvent(user, "custom_event", "value");
    statsig.logEvent(user, "custom_event", "value", metadata);

    Assertions.assertFalse(logger.warnLogs.isEmpty());
    for (String msg : logger.errorLogs) {
      Assertions.assertFalse(
          msg.contains("Failed to get entrySet"), "Unexpected error in logs: " + msg);
    }
  }

  static class TestLogger implements OutputLoggerProvider {
    public List<String> errorLogs = new ArrayList<>();
    public List<String> warnLogs = new ArrayList<>();

    @Override
    public void init() {}

    @Override
    public void debug(String tag, String msg) {}

    @Override
    public void info(String tag, String msg) {}

    @Override
    public void warn(String tag, String msg) {
      warnLogs.add(msg);
    }

    @Override
    public void error(String tag, String msg) {
      errorLogs.add(msg);
    }

    @Override
    public void shutdown() {}
  }
}
