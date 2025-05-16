package com.statsig;

import com.google.gson.JsonElement;

public class FailureDetails {
  public String reason;
  public JsonElement error;

  public String getReason() {
    return reason;
  }

  public JsonElement getError() {
    return error;
  }
}
