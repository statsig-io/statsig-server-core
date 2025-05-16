package com.statsig;

public class GetExperimentOptions {
  public boolean disableExposureLogging;

  public GetExperimentOptions(boolean disableExposureLogging) {
    this.disableExposureLogging = disableExposureLogging;
  }
}
