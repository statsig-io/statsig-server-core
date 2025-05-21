package com.statsig;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

@JsonIgnoreProperties(ignoreUnknown = true)
public class FailureDetails {
  public String reason;
  public Object error;

  /** Default constructor for Jackson deserialization. */
  public FailureDetails() {}

  public String getReason() {
    return reason;
  }

  public Object getError() {
    return error;
  }
}
