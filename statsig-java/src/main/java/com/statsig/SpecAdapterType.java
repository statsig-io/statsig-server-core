package com.statsig;

/**
 * Constants for the {@link SpecAdapterConfig#setAdapterType(String)} value.
 *
 * <p>These mirror the {@code SpecsAdapterType} variants recognized by the core SDK. Prefer these
 * constants over raw strings to avoid typos.
 */
public final class SpecAdapterType {
  private SpecAdapterType() {}

  public static final String DATA_STORE = "data_store";
  public static final String NETWORK_GRPC_WEBSOCKET = "network_grpc_websocket";
  public static final String NETWORK_HTTP = "network_http";
}
