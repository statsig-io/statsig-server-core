package com.statsig;

/**
 * Constants for the {@link SpecAdapterConfig#setAuthenticationMode(String)} value used by the gRPC
 * websocket spec adapter when connecting to a Forward Proxy.
 */
public final class AuthenticationMode {
  private AuthenticationMode() {}

  public static final String NONE = "none";
  public static final String TLS = "tls";
  public static final String MTLS = "mtls";
}
