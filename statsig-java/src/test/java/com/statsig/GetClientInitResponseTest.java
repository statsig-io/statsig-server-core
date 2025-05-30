package com.statsig;

import static org.junit.jupiter.api.Assertions.assertNotNull;

import java.util.HashMap;
import java.util.concurrent.ExecutionException;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class GetClientInitResponseTest {
  private Statsig statsigServer;
  private StatsigUser user;

  @BeforeEach
  public void setUp() throws ExecutionException, InterruptedException {
    statsigServer = new Statsig("secret-key");
    statsigServer.initialize().get();

    user =
        new StatsigUser.Builder()
            .setUserID("whd")
            .setEmail("weihao@statsig.com")
            .setCustomIDs(
                new HashMap<String, String>() {
                  {
                    put("k1", "v1");
                  }
                })
            .setCustom(
                new HashMap<String, Object>() {
                  {
                    put("k1", 1);
                  }
                })
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
    String res =
        statsigServer.getClientInitializeResponse(
            user, new ClientInitResponseOptions("client-key"));
    assertNotNull(res);
  }

  public void testGetClientInitResponseOptions() {
    ClientInitResponseOptions options = new ClientInitResponseOptions();
    options.setIncludeLocalOverrides(true);
    String res = statsigServer.getClientInitializeResponse(user, options);
    assertNotNull(res);
  }
}
