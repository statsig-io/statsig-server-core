package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;

import org.junit.jupiter.api.Test;

public class SpecAdapterConfigTest {

  @Test
  public void testSetterChainingAndGetters() {
    SpecAdapterConfig config =
        new SpecAdapterConfig()
            .setAdapterType("http")
            .setSpecsUrl("https://example.com/specs")
            .setInitTimeoutMs(2000L)
            .setAuthenticationMode("mtls")
            .setCaCertPath("/path/to/ca")
            .setClientCertPath("/path/to/client")
            .setClientKeyPath("/path/to/key")
            .setDomainName("example.com");

    assertEquals("http", config.getAdapterType());
    assertEquals("https://example.com/specs", config.getSpecsUrl());
    assertEquals(2000L, config.getInitTimeoutMs());
    assertEquals("mtls", config.getAuthenticationMode());
    assertEquals("/path/to/ca", config.getCaCertPath());
    assertEquals("/path/to/client", config.getClientCertPath());
    assertEquals("/path/to/key", config.getClientKeyPath());
    assertEquals("example.com", config.getDomainName());
  }

  @Test
  public void testSettersReturnSameInstance() {
    SpecAdapterConfig config = new SpecAdapterConfig();
    assertSame(config, config.setAdapterType("http"));
    assertSame(config, config.setSpecsUrl("https://example.com/specs"));
    assertSame(config, config.setInitTimeoutMs(1234L));
  }
}
