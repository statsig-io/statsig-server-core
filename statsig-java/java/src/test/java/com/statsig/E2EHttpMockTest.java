package com.statsig;

import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ExecutionException;

import static org.junit.jupiter.api.Assertions.*;

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

        boolean beforeOverride = statsig.checkGate(testUser, gateToTest);
        assertFalse(beforeOverride);

        statsig.overrideGate(gateToTest, true);
        boolean result = statsig.checkGate(testUser, gateToTest);
        assertTrue(result, "Feature gate should be enabled for test user");
    }

    @Test
    public void testDynamicConfig() {
        String configToTest = "test_icon_types";

        DynamicConfig configBeforeOverride = statsig.getDynamicConfig(testUser, configToTest);
        assertEquals(configBeforeOverride.value, new HashMap<>());
        
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

        Experiment expBeforeOverride = statsig.getExperiment(testUser, experimentToTest);
        assertEquals(expBeforeOverride.value, new HashMap<>());
        
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

    @Test
    public void testOverrideGateForSpecificUserAndRemove() {
        String gateToTest = "segment:segment_2_with_id_lists";

        StatsigUser user1 = testUser;
        StatsigUser user2 = new StatsigUser.Builder()
                .setUserID("different_user_id")
                .setEmail("other@example.com")
                .setCustom(Map.of("custom_field", "another_value"))
                .build();

        assertFalse(statsig.checkGate(user1, gateToTest));
        assertFalse(statsig.checkGate(user2, gateToTest));

        // Override for user1
        statsig.overrideGate(gateToTest, true, user1.getUserID());

        assertTrue(statsig.checkGate(user1, gateToTest));
        assertFalse(statsig.checkGate(user2, gateToTest));

        // Remove override for user1
        statsig.removeGateOverride(gateToTest, user1.getUserID());

        assertFalse(statsig.checkGate(user1, gateToTest));
        assertFalse(statsig.checkGate(user2, gateToTest));
    }

    @Test
    public void testOverrideDynamicConfigForSpecificUserAndRemove() {
        String configName = "test_icon_types";

        StatsigUser user1 = testUser;
        StatsigUser user2 = new StatsigUser.Builder()
                .setUserID("another_user_id")
                .build();

        assertEquals(statsig.getDynamicConfig(user1, configName).value, new HashMap<>());
        assertEquals(statsig.getDynamicConfig(user2, configName).value, new HashMap<>());

        Map<String, Object> overrideValue = Map.of(
                "color", "red",
                "size", 42
        );

        statsig.overrideDynamicConfig(configName, overrideValue, user1.getUserID());

        DynamicConfig config1 = statsig.getDynamicConfig(user1, configName);
        DynamicConfig config2 = statsig.getDynamicConfig(user2, configName);

        assertEquals("red", config1.getString("color", ""));
        assertEquals(42, config1.getInt("size", 0));
        assertEquals(config2.value, new HashMap<>());

        statsig.removeDynamicConfigOverride(configName, user1.getUserID());

        DynamicConfig config1AfterRemove = statsig.getDynamicConfig(user1, configName);
        assertEquals(config1AfterRemove.value, new HashMap<>());
    }

    @Test
    public void testOverrideExperimentForSpecificUserAndRemove() {
        String experimentName = "purchase_experiment";

        StatsigUser user1 = testUser;
        StatsigUser user2 = new StatsigUser.Builder()
                .setUserID("second_user_id")
                .build();

        assertEquals(statsig.getExperiment(user1, experimentName).value, new HashMap<>());
        assertEquals(statsig.getExperiment(user2, experimentName).value, new HashMap<>());

        Map<String, Object> overrideValue = Map.of(
                "price", 5.99,
                "highlight", true
        );

        statsig.overrideExperiment(experimentName, overrideValue, user1.getUserID());

        Experiment exp1 = statsig.getExperiment(user1, experimentName);
        Experiment exp2 = statsig.getExperiment(user2, experimentName);

        assertEquals(5.99, exp1.getDouble("price", 0.0));
        assertTrue(exp1.getBoolean("highlight", false));
        assertEquals(exp2.value, new HashMap<>());

        statsig.removeExperimentOverride(experimentName, user1.getUserID());

        Experiment exp1AfterRemove = statsig.getExperiment(user1, experimentName);
        assertEquals(exp1AfterRemove.value, new HashMap<>());
    }

    @Test
    public void testOverrideLayerForSpecificUserAndRemove() {
        String layerName = "layer_with_experiment";

        StatsigUser user1 = testUser;
        StatsigUser user2 = new StatsigUser.Builder()
                .setUserID("different_layer_user")
                .build();

        assertEquals(statsig.getLayer(user1, layerName).value, new HashMap<>());
        assertEquals(statsig.getLayer(user2, layerName).value, new HashMap<>());

        Map<String, Object> overrideValue = Map.of(
                "button_color", "blue",
                "animation", "bounce"
        );

        statsig.overrideLayer(layerName, overrideValue, user1.getUserID());

        Layer layer1 = statsig.getLayer(user1, layerName);
        Layer layer2 = statsig.getLayer(user2, layerName);

        assertEquals("blue", layer1.getString("button_color", ""));
        assertEquals("bounce", layer1.getString("animation", ""));
        assertEquals(layer2.value, new HashMap<>());

        statsig.removeLayerOverride(layerName, user1.getUserID());

        Layer layer1AfterRemove = statsig.getLayer(user1, layerName);
        assertEquals(layer1AfterRemove.value, new HashMap<>());
    }

    @Test
    public void testOverrideGateForCustomIDAndRemove() {
        String gateToTest = "segment:segment_2_with_id_lists";

        StatsigUser userWithCustomID = new StatsigUser.Builder()
                .setCustomIDs(Map.of("statsig_custom", "custom_id_1"))
                .build();

        StatsigUser otherUserWithCustomID = new StatsigUser.Builder()
                .setCustomIDs(Map.of("statsig_custom", "custom_id_2"))
                .build();

        assertFalse(statsig.checkGate(userWithCustomID, gateToTest));
        assertFalse(statsig.checkGate(otherUserWithCustomID, gateToTest));

        statsig.overrideGate(gateToTest, true, "custom_id_1");

        assertTrue(statsig.checkGate(userWithCustomID, gateToTest), "Gate should be true for custom_id_1 after override");
        assertFalse(statsig.checkGate(otherUserWithCustomID, gateToTest), "Gate should still be false for custom_id_2");

        statsig.removeGateOverride(gateToTest, "custom_id_1");

        assertFalse(statsig.checkGate(userWithCustomID, gateToTest));
        assertFalse(statsig.checkGate(otherUserWithCustomID, gateToTest));
    }

}
