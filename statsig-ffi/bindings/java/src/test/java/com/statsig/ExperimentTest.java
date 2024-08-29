package com.statsig;

import com.google.gson.Gson;
import com.statsig.internal.GsonUtil;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import static org.junit.jupiter.api.Assertions.*;

public class ExperimentTest {

    private final Gson gson = GsonUtil.getGson();

    @Test
    public void testDeserialization() throws IOException {
        String json = TestUtils.loadJsonFromFile("experiment.json");

        Experiment experiment = gson.fromJson(json, Experiment.class);

        assertNotNull(experiment);
        assertEquals("Test Experiment", experiment.name);
        assertEquals("rule-123", experiment.ruleID);
        assertEquals("test-group", experiment.groupName);

        String expectedStringValue = "value1";
        int expectedIntValue = 2;

        assertEquals(expectedStringValue, experiment.getString("key1", "defaultValue"));
        assertEquals(expectedIntValue, experiment.getInt("key2", -1));
    }
}
