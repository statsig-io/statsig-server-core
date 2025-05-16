package com.statsig;

import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.time.temporal.ChronoUnit;

public class OutputLogger {
  public enum LogLevel {
    NONE(0),
    ERROR(1),
    WARN(2),
    INFO(3),
    DEBUG(4);

    private final int value;

    LogLevel(final int newValue) {
      value = newValue;
    }

    public int getValue() {
      return value;
    }

    public String getLevelString() {
      switch (this) {
        case ERROR:
          return "ERROR";
        case WARN:
          return "WARN";
        case INFO:
          return "INFO";
        case DEBUG:
          return "DEBUG";
        default:
          return "";
      }
    }
  }

  static LogLevel logLevel = LogLevel.WARN;

  static void logError(String tag, String message) {
    logMessage(LogLevel.ERROR, tag, message);
  }

  static void logWarning(String tag, String message) {
    logMessage(LogLevel.WARN, tag, message);
  }

  static void logInfo(String tag, String message) {
    logMessage(LogLevel.INFO, tag, message);
  }

  static void logMessage(LogLevel level, String tag, String message) {
    if (level.getValue() > logLevel.getValue()) {
      return;
    }

    String timestamp =
        DateTimeFormatter.ISO_INSTANT.format(Instant.now().truncatedTo(ChronoUnit.MILLIS));
    System.out.printf(
        "%s %s [com.statsig.OutputLogger] [Statsig.%s] %s%n",
        timestamp, level.getLevelString(), tag, message);
  }
}
