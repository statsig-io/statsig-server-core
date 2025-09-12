package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;

public class FailureDetails {
  public final String reason;
  public final Object error;

  @JSONCreator
  FailureDetails(
      @JSONField(name = "reason") String reason, @JSONField(name = "error") Object error) {
    this.reason = reason;
    this.error = error;
  }

  public String getReason() {
    return reason;
  }

  public Object getError() {
    return error;
  }
}
