package com.statsig;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.statsig.internal.HasRawJson;

public class InitializeDetails implements HasRawJson {
  public double duration;

  @JsonProperty("init_success")
  public boolean initSuccess;

  @JsonProperty("is_config_spec_ready")
  public boolean isConfigSpecReady;

  @JsonProperty("is_id_list_ready")
  public Boolean isIdListReady;

  public String source;

  @JsonProperty("failure_details")
  public FailureDetails failureDetails;

  @JsonIgnore private String rawJson;

  /** Default constructor for Jackson deserialization. */
  public InitializeDetails() {}

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

  public void setRawJson(String rawJson) {
    this.rawJson = rawJson;
  }
}
