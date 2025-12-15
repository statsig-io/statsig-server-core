package com.statsig;

import java.util.Map;

public class GetExperimentOptions {
  public boolean disableExposureLogging;
  public Map<String, StickyValues> userPersistedValues;

  public GetExperimentOptions(Map<String, StickyValues> userPersistedValues) {
    this.userPersistedValues = userPersistedValues;
  }

  public GetExperimentOptions(boolean disableExposureLogging) {
    this.disableExposureLogging = disableExposureLogging;
  }

  public GetExperimentOptions(
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
