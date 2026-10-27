package com.statsig;

import static org.junit.jupiter.api.Assertions.*;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONArray;
import com.alibaba.fastjson2.JSONObject;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ExecutionException;
import java.util.zip.GZIPInputStream;
import okhttp3.mockwebserver.Dispatcher;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import okhttp3.mockwebserver.RecordedRequest;
import org.jetbrains.annotations.NotNull;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/** Java uses the raw experiment path (use_raw_experiment_with_options) through JNI. */
public class CmabExposureRuleIdTest {
  private static final String DCS_PATH =
      "../statsig-rust/tests/data/dcs_cmab_exposure_rule_id.json";

  private MockWebServer mockWebServer;
  private Statsig statsig;
  private final List<String> loggedRuleIds = new ArrayList<>();

  @BeforeEach
  public void setUp() throws IOException, InterruptedException, ExecutionException {
    String dcs = new String(Files.readAllBytes(Paths.get(DCS_PATH)), StandardCharsets.UTF_8);

    mockWebServer = new MockWebServer();
    mockWebServer.setDispatcher(
        new Dispatcher() {
          @NotNull
          @Override
          public MockResponse dispatch(@NotNull RecordedRequest request) {
            String path = request.getPath() == null ? "" : request.getPath();
            if (path.contains("download_config_specs")) {
              return new MockResponse().setResponseCode(200).setBody(dcs);
            }
            if (path.contains("log_event")) {
              recordExposureRuleIds(request);
              return new MockResponse().setResponseCode(202).setBody("{\"success\": true}");
            }
            return new MockResponse().setResponseCode(404);
          }
        });
    mockWebServer.start();

    StatsigOptions options =
        new StatsigOptions.Builder()
            .setSpecsUrl(mockWebServer.url("/v2/download_config_specs").toString())
            .setLogEventUrl(mockWebServer.url("/v1/log_event").toString())
            .build();

    statsig = new Statsig("secret-cmab-rule-id-test", options);
    statsig.initialize().get();
  }

  @AfterEach
  public void tearDown() throws IOException {
    mockWebServer.shutdown();
  }

  @Test
  public void testExploreSuffixPreservedInAutomaticExposure()
      throws ExecutionException, InterruptedException {
    StatsigUser user = new StatsigUser.Builder().setUserID("raw-explore-user").build();

    Experiment experiment = statsig.getExperiment(user, "cmab_always_explore");
    statsig.shutdown().get();

    assertTrue(experiment.ruleID.endsWith(":explore"), experiment.ruleID);
    assertEquals(experiment.ruleID, getLoggedRuleId());
  }

  @Test
  public void testExploitRuleIdRemainsBareInAutomaticExposure()
      throws ExecutionException, InterruptedException {
    StatsigUser user = new StatsigUser.Builder().setUserID("raw-exploit-user").build();

    Experiment experiment = statsig.getExperiment(user, "cmab_never_explore");
    statsig.shutdown().get();

    assertEquals("treatment-group-id", experiment.ruleID);
    assertEquals(experiment.ruleID, getLoggedRuleId());
  }

  private String getLoggedRuleId() {
    synchronized (loggedRuleIds) {
      assertEquals(1, loggedRuleIds.size(), loggedRuleIds.toString());
      return loggedRuleIds.get(0);
    }
  }

  private void recordExposureRuleIds(RecordedRequest request) {
    JSONArray events = JSON.parseObject(readBody(request)).getJSONArray("events");
    if (events == null) {
      return;
    }

    synchronized (loggedRuleIds) {
      for (int i = 0; i < events.size(); i++) {
        JSONObject event = events.getJSONObject(i);
        if ("statsig::config_exposure".equals(event.getString("eventName"))) {
          loggedRuleIds.add(event.getJSONObject("metadata").getString("ruleID"));
        }
      }
    }
  }

  private static String readBody(RecordedRequest request) {
    byte[] bytes = request.getBody().readByteArray();
    if (!"gzip".equals(request.getHeader("Content-Encoding"))) {
      return new String(bytes, StandardCharsets.UTF_8);
    }

    try (InputStream in = new GZIPInputStream(new ByteArrayInputStream(bytes))) {
      ByteArrayOutputStream out = new ByteArrayOutputStream();
      byte[] buffer = new byte[4096];
      int read;
      while ((read = in.read(buffer)) != -1) {
        out.write(buffer, 0, read);
      }
      return new String(out.toByteArray(), StandardCharsets.UTF_8);
    } catch (IOException e) {
      throw new RuntimeException(e);
    }
  }
}
