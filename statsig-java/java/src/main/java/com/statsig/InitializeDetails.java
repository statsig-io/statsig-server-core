package com.statsig;

import com.google.gson.annotations.Expose;
import com.google.gson.annotations.SerializedName;

public class InitializeDetails {
  public double duration;

  @SerializedName("init_success")
  public boolean initSuccess;

  @SerializedName("is_config_spec_ready")
  public boolean isConfigSpecReady;

  @SerializedName("is_id_list_ready")
  public Boolean isIdListReady;

  public String source;

  @SerializedName("failure_details")
  public FailureDetails failureDetails;

  @Expose(serialize = false, deserialize = false)
  String rawJson;

  InitializeDetails(
      double duration,
      boolean initSuccess,
      boolean isConfigSpecReady,
      Boolean isIdListReady,
      String source,
      FailureDetails failureDetails) {
    this.duration = duration;
    this.initSuccess = initSuccess;
    this.isConfigSpecReady = isConfigSpecReady;
    this.isIdListReady = isIdListReady;
    this.source = source;
    this.failureDetails = failureDetails;
  }

  public double getDuration() {
    return duration;
  }

  public boolean getIsInitSuccess() {
    return initSuccess;
  }

  public boolean getIsConfigSpecReady() {
    return isConfigSpecReady;
  }

  public boolean getIsIdListReady() {
    return isIdListReady != null ? isIdListReady : false;
  }

  public String getSource() {
    return source;
  }

  public FailureDetails getFailureDetails() {
    return failureDetails;
  }

  public String getRawJson() {
    return rawJson;
  }

  void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }
}
