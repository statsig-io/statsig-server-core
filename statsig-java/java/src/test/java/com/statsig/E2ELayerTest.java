package com.statsig;

import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.Map;
import java.util.concurrent.ExecutionException;

import static org.junit.jupiter.api.Assertions.*;

/**
 * End-to-end tests for Layer functionality with HTTP mocking.
 */
public class E2ELayerTest {
    private MockWebServer mockWebServer;
    private Statsig statsig;
    private StatsigUser testUser;
    private String downloadConfigSpecsJson;

    @BeforeEach
    public void setUp() throws IOException, InterruptedException, ExecutionException {
        downloadConfigSpecsJson = TestUtils.loadJsonFromFile("download_config_specs.json");

        mockWebServer = new MockWebServer();
        mockWebServer.start();

        mockWebServer.enqueue(new MockResponse()
                .setResponseCode(200)
                .setHeader("Content-Type", "application/json")
                .setBody(downloadConfigSpecsJson));

        testUser = new StatsigUser.Builder()
                .setUserID("test_user_id")
                .setEmail("test@example.com")
                .setCustom(Map.of("custom_field", "custom_value"))
                .build();

        StatsigOptions options = new StatsigOptions.Builder()
                .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
                .setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG)
                .build();

        statsig = new Statsig("secret-test-key", options);
        statsig.initialize().get();
    }

    @AfterEach
    public void tearDown() throws IOException, ExecutionException, InterruptedException {
        if (statsig != null) {
            statsig.shutdown().get();
        }
        mockWebServer.shutdown();
    }

    @Test
    public void testLayer() {
        String layerToTest = "a_layer";

        Layer layerBeforeOverride = statsig.getLayer(testUser, layerToTest);
        assertNotNull(layerBeforeOverride);
        assertEquals("red", layerBeforeOverride.getString("button_color", "red"));
        assertEquals("Purchase", layerBeforeOverride.getString("button_text", "Purchase"));
        assertEquals(0, layerBeforeOverride.getInt("discount_percentage", 0));
        
        Map<String, Object> layerValue = Map.of(
            "button_color", "blue",
            "button_text", "Buy Now",
            "discount_percentage", 15
        );
        
        statsig.overrideLayer(layerToTest, layerValue);
        
        Layer layer = statsig.getLayer(testUser, layerToTest);
        
        assertNotNull(layer);
        assertEquals("blue", layer.getString("button_color", "red"));
        assertEquals("Buy Now", layer.getString("button_text", "Purchase"));
        assertEquals(15, layer.getInt("discount_percentage", 0));
    }
}
