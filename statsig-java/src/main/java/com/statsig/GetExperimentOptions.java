package com.statsig;

import java.util.Map;

public class GetExperimentOptions {
  public boolean disableExposureLogging;
  public Map<String, StickyValues> userPersistedValues;

  /**
   * When a persisted sticky value exists, let a matching console override rule take precedence over
   * it.
   */
  public boolean enforceOverrides;

  /**
   * When a persisted sticky value exists, re-check targeting and drop the sticky value if the user
   * no longer passes targeting.
   */
  public boolean enforceTargeting;

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

  public GetExperimentOptions(
      boolean disableExposureLogging,
      Map<String, StickyValues> userPersistedValues,
      boolean enforceOverrides,
      boolean enforceTargeting) {
    this.disableExposureLogging = disableExposureLogging;
    this.userPersistedValues = userPersistedValues;
    this.enforceOverrides = enforceOverrides;
    this.enforceTargeting = enforceTargeting;
  }

  public boolean getDisableExposureLogging() {
    return disableExposureLogging;
  }

  public Map<String, StickyValues> getUserPersistedValues() {
    return userPersistedValues;
  }

  public boolean getEnforceOverrides() {
    return enforceOverrides;
  }

  public boolean getEnforceTargeting() {
    return enforceTargeting;
  }
}
