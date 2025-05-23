package com.statsig;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

public class OutputLoggerProviderTest {

  public static class MockOutputLoggerProvider implements OutputLoggerProvider {
    public List<String> calledMethods = new ArrayList<>();
    public List<LogMessage> logMessages = new ArrayList<>();

    public static class LogMessage {
      public final String level;
      public final String tag;
      public final String message;

      public LogMessage(String level, String tag, String message) {
        this.level = level;
        this.tag = tag;
        this.message = message;
      }
    }

    @Override
    public void init() {
      calledMethods.add("init");
    }

    @Override
    public void debug(String tag, String msg) {
      calledMethods.add("debug");
      logMessages.add(new LogMessage("debug", tag, msg));
    }

    @Override
    public void info(String tag, String msg) {
      calledMethods.add("info");
      logMessages.add(new LogMessage("info", tag, msg));
    }

    @Override
    public void warn(String tag, String msg) {
      calledMethods.add("warn");
      logMessages.add(new LogMessage("warn", tag, msg));
    }

    @Override
    public void error(String tag, String msg) {
      calledMethods.add("error");
      logMessages.add(new LogMessage("error", tag, msg));
    }

    @Override
    public void shutdown() {
      calledMethods.add("shutdown");
    }
  }

  @Test
  public void testOutputLoggerProviderUsage() throws Exception {
    MockOutputLoggerProvider mockProvider = new MockOutputLoggerProvider();

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setOutputLoggerProvider(mockProvider)
            .setOutputLoggerLevel(
                OutputLogger.LogLevel.DEBUG) // Set to DEBUG to capture all log levels
            .setSpecsSyncIntervalMs(1) // Set to 1ms to trigger logs quickly
            .build();

    Statsig statsig = new Statsig("secret-key", options);
    statsig.initialize().get();

    StatsigUser user =
        new StatsigUser.Builder().setUserID("123").setEmail("test@example.com").build();

    statsig.checkGate(user, "test-gate");

    statsig.flushEvents().get();
    statsig.shutdown().get();

    assertTrue(mockProvider.calledMethods.contains("init"), "Expected init method to be called");
    assertTrue(
        mockProvider.calledMethods.contains("debug")
            || mockProvider.calledMethods.contains("info")
            || mockProvider.calledMethods.contains("warn")
            || mockProvider.calledMethods.contains("error"),
        "Expected at least one log method to be called");
    assertTrue(
        mockProvider.calledMethods.contains("shutdown"), "Expected shutdown method to be called");

    assertNotNull(mockProvider.logMessages);
    assertTrue(mockProvider.logMessages.size() > 0, "Expected log messages to be recorded");
  }

  private MockOutputLoggerProvider.LogMessage findLogMessage(
      List<MockOutputLoggerProvider.LogMessage> messages, String containsText) {
    for (MockOutputLoggerProvider.LogMessage msg : messages) {
      if (msg.message.contains(containsText)) {
        return msg;
      }
    }
    return null;
  }
}
