package com.statsig;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertTrue;

public class NativeLibraryLoaderTest {
    @Test
    public void testLoadNativeLibrary() {
        assertTrue(StatsigJNI.isLibraryLoaded());
    }
}
