package com.statsig;

import java.util.Map;

public class GetLayerOptions {
  public boolean disableExposureLogging;
  public Map<String, StickyValues> userPersistedValues;

  public GetLayerOptions(boolean disableExposureLogging) {
    this.disableExposureLogging = disableExposureLogging;
  }

  public GetLayerOptions(Map<String, StickyValues> userPersistedValues) {
    this.userPersistedValues = userPersistedValues;
  }

  public GetLayerOptions(
      boolean disableExposureLogging, Map<String, StickyValues> userPersistedValues) {
    this.disableExposureLogging = disableExposureLogging;
    this.userPersistedValues = userPersistedValues;
  }

  public boolean getDisableExposureLogging() {
    return disableExposureLogging;
  }

  public Map<String, StickyValues> getUserPersistedValues() {
    return userPersistedValues;
  }
}
