package com.statsig;

public class DataStoreBytesResponse {
  public byte[] result;
  public Long time;

  public DataStoreBytesResponse() {}

  public DataStoreBytesResponse(byte[] result, Long time) {
    this.result = result;
    this.time = time;
  }

  public byte[] getResult() {
    return result;
  }

  public Long getTime() {
    return time;
  }
}
