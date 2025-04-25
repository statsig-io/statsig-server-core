package com.statsig;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.statsig.internal.GsonUtil;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.Map;
import java.util.concurrent.ExecutionException;

import static org.junit.jupiter.api.Assertions.*;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

/**
 * End-to-end tests with HTTP mocking.
 * These tests use OkHttp's MockWebServer to intercept HTTP requests and
 * return predefined responses from download_config_specs.json.
 */
public class E2EHttpMockTest {
    private MockWebServer mockWebServer;
    private Statsig statsig;
    private StatsigUser testUser;
    private String downloadConfigSpecsJson;
    private final Gson gson = new GsonBuilder().setLenient().create();

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
    public void testFeatureGate() {
        String gateToTest = "segment:segment_2_with_id_lists";
        
        statsig.overrideGate(gateToTest, true);
        
        boolean result = statsig.checkGate(testUser, gateToTest);
        assertTrue(result, "Feature gate should be enabled for test user");
    }

    @Test
    public void testDynamicConfig() {
        String configToTest = "test_icon_types";
        
        Map<String, Object> configValue = Map.of(
            "key1", "value1",
            "key2", 123
        );
        
        statsig.overrideDynamicConfig(configToTest, configValue);
        
        DynamicConfig config = statsig.getDynamicConfig(testUser, configToTest);
        
        assertNotNull(config);
        assertEquals("value1", config.getString("key1", "default"));
        assertEquals(123, config.getInt("key2", 0));
    }

    @Test
    public void testExperiment() {
        String experimentToTest = "purchase_experiment";
        
        Map<String, Object> experimentValue = Map.of(
            "price", 9.99,
            "showDiscount", true
        );
        
        statsig.overrideExperiment(experimentToTest, experimentValue);
        
        Experiment experiment = statsig.getExperiment(testUser, experimentToTest);
        
        assertNotNull(experiment);
        assertEquals(9.99, experiment.getDouble("price", 0.0));
        assertTrue(experiment.getBoolean("showDiscount", false));
    }
}
