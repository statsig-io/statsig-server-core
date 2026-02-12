package com.statsig;

import com.alibaba.fastjson2.annotation.JSONCreator;
import com.alibaba.fastjson2.annotation.JSONField;
import java.util.HashMap;
import java.util.Map;

public class EvaluationDetails {
  /** last config updated time */
  public final long lcut;

  /** The time when we received this config */
  public final long receivedAt;

  /** Evaluation reason */
  public final String reason;

  @JSONCreator
  EvaluationDetails(
      @JSONField(name = "lcut") long lcut,
      @JSONField(name = "receivedAt") long receivedAt,
      @JSONField(name = "reason") String reason) {
    this.lcut = lcut;
    this.receivedAt = receivedAt;
    this.reason = reason;
  }

  public String getReason() {
    return reason;
  }

  public long getLcut() {
    return lcut;
  }

  public long getReceivedAt() {
    return receivedAt;
  }

  @Override
  public String toString() {
    return String.format(
        "EvaluationDetails { lcut=%s, receivedAt=%s, reason='%s' }", lcut, receivedAt, reason);
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("lcut", lcut);
    map.put("receivedAt", receivedAt);
    map.put("reason", reason);
    return map;
  }
}
