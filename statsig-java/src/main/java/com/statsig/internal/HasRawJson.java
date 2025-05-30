package com.statsig.internal;

/** Interface for objects that have a rawJson field. */
public interface HasRawJson {
  void setRawJson(String rawJson);

  String getRawJson();
}
