package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import com.statsig.internal.HasRawJson;

public class InitializeDetails implements HasRawJson {
  public double duration;
  public boolean initSuccess;
  public boolean isConfigSpecReady;
  public Boolean isIdListReady;
  public String source;
  public FailureDetails failureDetails;

  @JSONField(deserialize = false)
  private String rawJson;

  @JSONCreator
  InitializeDetails(
      @JSONField(name = "duration") double duration,
      @JSONField(name = "init_success") boolean initSuccess,
      @JSONField(name = "is_config_spec_ready") boolean isConfigSpecReady,
      @JSONField(name = "is_id_list_ready") Boolean isIdListReady,
      @JSONField(name = "source") String source,
      @JSONField(name = "failure_details") FailureDetails failureDetails) {
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
