package com.statsig;

import com.google.gson.Gson;
import com.statsig.internal.GsonUtil;
import org.junit.jupiter.api.Test;

import java.io.IOException;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

public class FeatureGateTest {
    private static final Gson gson = GsonUtil.getGson();

    @Test
    public void testGateDeserialization() throws IOException {
        String json = TestUtils.loadJsonFromFile("gate.json");

        FeatureGate gate = gson.fromJson(json, FeatureGate.class);

        assertEquals("cPaGkTiRBuP1SfwoxRtTvhT6Vxpa4v/342Z1N0pXUlc=", gate.name);
        assertEquals("6X3qJgyfwA81IJ2dxI7lYp17", gate.ruleID);
        assertTrue(gate.value);
    }
}
