package com.statsig;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.Map;
import java.util.concurrent.ExecutionException;

import static org.junit.jupiter.api.Assertions.assertNotNull;

public class GetClientInitResponseTest {
    private Statsig statsigServer;
    private StatsigUser user;

    @BeforeEach
    public void setUp() throws ExecutionException, InterruptedException {
        statsigServer = new Statsig("secret-key");
        statsigServer.initialize().get();

        user = new StatsigUser.Builder()
                .setUserID("whd")
                .setEmail("weihao@statsig.com")
                .setCustomIDs(Map.of("k1", "v1"))
                .setCustom(Map.of("k1", 1))
                .build();
    }

    @AfterEach
    public void tearDown() throws ExecutionException, InterruptedException {
        statsigServer.shutdown().get();
    }

    @Test
    public void testGetClientInitResponse() {
        String res = statsigServer.getClientInitializeResponse(user);
        assertNotNull(res);
    }

    @Test
    public void testGetClientInitResponseWithTargetApp() {
        String res = statsigServer.getClientInitializeResponse(user, new ClientInitResponseOptions("client-key"));
        assertNotNull(res);
    }
}
