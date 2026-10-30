package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import com.alibaba.fastjson2.JSON;
import com.statsig.experimental.*;
import java.util.*;
import java.util.concurrent.*;
import okhttp3.mockwebserver.*;
import org.junit.jupiter.api.Test;

public class UserEnrichmentConcurrencyTest {
  static final String SPECS =
      "{\"time\":1,\"has_updates\":true,\"feature_gates\":[],\"dynamic_configs\":[],\"layer_configs\":[]}";

  static ForwardProxyUserEnrichmentProvider provider(MockWebServer server) {
    ForwardProxyEnrichmentOptions options =
        new ForwardProxyEnrichmentOptions(server.url("/").toString());
    options.timeoutMs = 5000L;
    options.retries = 0;
    return new ForwardProxyUserEnrichmentProvider("secret-test", options);
  }

  static Statsig sdk(MockWebServer server) {
    return new Statsig(
        "secret-test",
        new StatsigOptions.Builder()
            .setSpecsUrl(server.url("/specs").toString())
            .setDisableAllLogging(true)
            .setFallbackToStatsigApi(false)
            .build());
  }

  @Test
  public void overlappingBuildsPreserveNativeIdentity() throws Exception {
    try (MockWebServer server = new MockWebServer()) {
      server.start();
      server.enqueue(new MockResponse().setBody(SPECS));
      Statsig sdk = sdk(server);
      sdk.initialize().get(10, TimeUnit.SECONDS);
      try {
        StatsigFPUserBuilder builder = new StatsigFPUserBuilder(provider(server));
        List<CompletableFuture<StatsigUser>> futures = new ArrayList<>();
        for (int i = 0; i < 128; i++) {
          server.enqueue(new MockResponse().setBody("{\"id_lists\":{}}"));
          futures.add(builder.setUserID("user-" + i).build());
        }
        for (int i = 0; i < futures.size(); i++) {
          StatsigUser user = futures.get(i).get(10, TimeUnit.SECONDS);
          try {
            assertEquals("user-" + i, user.getUserID());
            String nativeID =
                JSON.parseObject(
                        sdk.getClientInitializeResponse(
                            user, new ClientInitResponseOptions(HashAlgo.NONE)))
                    .getJSONObject("evaluated_keys")
                    .getString("userID");
            assertEquals(user.getUserID(), nativeID);
          } finally {
            user.close();
          }
        }
      } finally {
        sdk.shutdown().get(10, TimeUnit.SECONDS);
      }
    }
  }

  @Test
  public void asyncBuildCompletesWhileCommonPoolIsOccupied() throws Exception {
    ForkJoinPool pool = ForkJoinPool.commonPool();
    int workers = pool.getParallelism();
    CountDownLatch occupied = new CountDownLatch(workers), release = new CountDownLatch(1);
    for (int i = 0; i < workers; i++)
      pool.execute(
          () -> {
            occupied.countDown();
            try {
              release.await();
            } catch (InterruptedException e) {
              Thread.currentThread().interrupt();
            }
          });
    try (MockWebServer server = new MockWebServer()) {
      server.start();
      assertTrue(occupied.await(5, TimeUnit.SECONDS));
      server.enqueue(new MockResponse().setBody("{\"id_lists\":{}}"));
      StatsigUser user =
          new StatsigFPUserBuilder(provider(server))
              .setUserID("async")
              .build()
              .get(3, TimeUnit.SECONDS);
      user.close();
    } finally {
      release.countDown();
    }
  }

  @Test
  public void blockingBuildInSdkCallbackDoesNotAbortJvm() throws Exception {
    java.io.File output = java.io.File.createTempFile("fp-callback", ".log");
    Process process =
        new ProcessBuilder(
                System.getProperty("java.home") + "/bin/java",
                "-cp",
                System.getProperty("statsig.test.classpath", System.getProperty("java.class.path")),
                CallbackProbe.class.getName())
            .redirectErrorStream(true)
            .redirectOutput(output)
            .start();
    try {
      assertTrue(process.waitFor(20, TimeUnit.SECONDS), "callback process hung");
      assertEquals(
          0,
          process.exitValue(),
          new String(
              java.nio.file.Files.readAllBytes(output.toPath()),
              java.nio.charset.StandardCharsets.UTF_8));
    } finally {
      process.destroyForcibly();
      output.delete();
    }
  }

  public static class CallbackProbe {
    public static void main(String[] args) throws Exception {
      try (MockWebServer server = new MockWebServer()) {
        server.start();
        server.enqueue(new MockResponse().setBodyDelay(200, TimeUnit.MILLISECONDS).setBody(SPECS));
        server.enqueue(new MockResponse().setBody("{\"id_lists\":{}}"));
        Statsig sdk = sdk(server);
        StatsigFPUserBuilder builder =
            new StatsigFPUserBuilder(provider(server)).setUserID("callback");
        sdk.initialize()
            .thenRun(
                () -> {
                  StatsigUser user = builder.buildBlocking();
                  assertEquals("callback", user.getUserID());
                  user.close();
                })
            .get(10, TimeUnit.SECONDS);
        sdk.shutdown().get(10, TimeUnit.SECONDS);
      }
      System.exit(0);
    }
  }
}
