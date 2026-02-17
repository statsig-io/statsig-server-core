package com.statsig.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

public class NativeBinaryResolverTest {

  @Test
  public void testParseBooleanOverrideTrueValues() {
    assertEquals(Boolean.TRUE, NativeBinaryResolver.parseBooleanOverride("true"));
    assertEquals(Boolean.TRUE, NativeBinaryResolver.parseBooleanOverride("TRUE"));
    assertEquals(Boolean.TRUE, NativeBinaryResolver.parseBooleanOverride("1"));
  }

  @Test
  public void testParseBooleanOverrideFalseValues() {
    assertEquals(Boolean.FALSE, NativeBinaryResolver.parseBooleanOverride("false"));
    assertEquals(Boolean.FALSE, NativeBinaryResolver.parseBooleanOverride("FALSE"));
  }
}
