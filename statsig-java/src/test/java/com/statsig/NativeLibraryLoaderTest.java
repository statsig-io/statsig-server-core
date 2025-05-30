package com.statsig;

import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

public class NativeLibraryLoaderTest {

  // no-op for most of the time
  // unless you change the way of loading native lib
  // you want to test again
  @Test
  public void testLoadNativeLibrary() {
    assertTrue(StatsigJNI.isLibraryLoaded());
  }
}
