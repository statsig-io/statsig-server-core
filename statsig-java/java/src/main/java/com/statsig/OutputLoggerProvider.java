package com.statsig;

public interface OutputLoggerProvider {
  void init();

  void debug(String tag, String msg);

  void info(String tag, String msg);

  void warn(String tag, String msg);

  void error(String tag, String msg);

  void shutdown();
}
