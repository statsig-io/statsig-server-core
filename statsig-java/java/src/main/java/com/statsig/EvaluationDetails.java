package com.statsig;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.HashMap;
import java.util.Map;

public class EvaluationDetails {
  /** last config updated time */
  public long lcut;

  /** The time when we received this config */
  @JsonProperty("received_at")
  public long receivedAt;

  /** Evaluation reason */
  public String reason;

  /** Default constructor for Jackson deserialization. */
  public EvaluationDetails() {}

  public String getReason() {
    return reason;
  }

  public long getLcut() {
    return lcut;
  }

  public long getReceivedAt() {
    return receivedAt;
  }

  EvaluationDetails(long lcut, long receivedAt, String reason) {
    this.lcut = lcut;
    this.receivedAt = receivedAt;
    this.reason = reason;
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
