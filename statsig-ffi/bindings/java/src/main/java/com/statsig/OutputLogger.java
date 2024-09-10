package com.statsig;

import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.time.temporal.ChronoUnit;

public class OutputLogger {
    public enum LogLevel {
        NONE(0),
        DEBUG(1),
        INFO(2),
        WARN(3),
        ERROR(4);

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

    static void logError(String context, String message) {
        logMessage(LogLevel.ERROR, context, message);
    }

    static void logWarning(String context, String message) {
        logMessage(LogLevel.WARN, context, message);
    }

    static void logInfo(String context, String message) {
        logMessage(LogLevel.INFO, context, message);
    }

    static void logMessage(LogLevel level, String context, String message) {
        if (level.getValue() < logLevel.getValue()) {
            return;
        }

        String timestamp = DateTimeFormatter.ISO_INSTANT.format(Instant.now().truncatedTo(ChronoUnit.MILLIS));
        System.out.printf("%s %s [%s] [Statsig] %s%n", timestamp, level.getLevelString(), context, message);
    }
}
