package com.statsig;

public class DataStoreResponse {
  public String result;
  public Long time;

  public DataStoreResponse() {}

  public DataStoreResponse(String result, Long time) {
    this.result = result;
    this.time = time;
  }

  public String getResult() {
    return result;
  }

  public Long getTime() {
    return time;
  }

  @Override
  public String toString() {
    return "DataStoreResponse{" + "result='" + result + '\'' + ", time=" + time + '}';
  }
}
