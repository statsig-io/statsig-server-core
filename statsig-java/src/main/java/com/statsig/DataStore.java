package com.statsig;

import java.util.concurrent.CompletableFuture;

public interface DataStore {
  CompletableFuture<Void> initialize();

  CompletableFuture<Void> shutdown();

  CompletableFuture<DataStoreResponse> get(String key);

  default CompletableFuture<DataStoreBytesResponse> getBytes(String key) {
    return bytesNotImplementedFuture();
  }

  CompletableFuture<Void> set(String key, String value, Long time);

  default CompletableFuture<Void> setBytes(String key, byte[] value, Long time) {
    return bytesNotImplementedFuture();
  }

  CompletableFuture<Boolean> supportPollingUpdatesFor(String path);

  static <T> CompletableFuture<T> bytesNotImplementedFuture() {
    CompletableFuture<T> future = new CompletableFuture<>();
    future.completeExceptionally(new UnsupportedOperationException("Bytes method not implemented"));
    return future;
  }
}
