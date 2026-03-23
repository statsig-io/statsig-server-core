package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import org.junit.jupiter.api.Test;

public class SpecAdapterConfigTest {

  @Test
  public void testSetterChainingAndGetters() {
    SpecAdapterConfig config =
        new SpecAdapterConfig()
            .setAdapterType(SpecAdapterType.NETWORK_HTTP)
            .setSpecsUrl("https://example.com/specs")
            .setInitTimeoutMs(2000L)
            .setAuthenticationMode(AuthenticationMode.MTLS)
            .setCaCertPath("/path/to/ca")
            .setClientCertPath("/path/to/client")
            .setClientKeyPath("/path/to/key")
            .setDomainName("example.com");

    assertEquals(SpecAdapterType.NETWORK_HTTP, config.getAdapterType());
    assertEquals("https://example.com/specs", config.getSpecsUrl());
    assertEquals(2000L, config.getInitTimeoutMs());
    assertEquals(AuthenticationMode.MTLS, config.getAuthenticationMode());
    assertEquals("/path/to/ca", config.getCaCertPath());
    assertEquals("/path/to/client", config.getClientCertPath());
    assertEquals("/path/to/key", config.getClientKeyPath());
    assertEquals("example.com", config.getDomainName());
  }

  @Test
  public void testSettersReturnSameInstance() {
    SpecAdapterConfig config = new SpecAdapterConfig();
    assertSame(config, config.setAdapterType(SpecAdapterType.NETWORK_HTTP));
    assertSame(config, config.setSpecsUrl("https://example.com/specs"));
    assertSame(config, config.setInitTimeoutMs(1234L));
  }

  @Test
  public void testTypedConstantsExposeCanonicalValues() {
    assertEquals("data_store", SpecAdapterType.DATA_STORE);
    assertEquals("network_http", SpecAdapterType.NETWORK_HTTP);
    assertEquals("network_grpc_websocket", SpecAdapterType.NETWORK_GRPC_WEBSOCKET);
    assertEquals("none", AuthenticationMode.NONE);
    assertEquals("tls", AuthenticationMode.TLS);
    assertEquals("mtls", AuthenticationMode.MTLS);
  }
}
