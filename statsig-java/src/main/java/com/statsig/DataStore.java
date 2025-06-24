package com.statsig;

import java.util.concurrent.CompletableFuture;

public interface DataStore {
  CompletableFuture<Void> initialize();

  CompletableFuture<Void> shutdown();

  CompletableFuture<DataStoreResponse> get(String key);

  CompletableFuture<Void> set(String key, String value, Long time);

  CompletableFuture<Boolean> supportPollingUpdatesFor(String path);
}
