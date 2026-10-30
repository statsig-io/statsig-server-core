package com.statsig;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.statsig.experimental.EnrichmentException;
import com.statsig.experimental.ForwardProxyEnrichmentOptions;
import com.statsig.experimental.ForwardProxyUserEnrichmentProvider;
import com.statsig.experimental.StatsigFPUserBuilder;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

public class UserEnrichmentTest {
  private MockWebServer mockWebServer;

  @Test
  public void testApiIsOnlyInExperimentalPackage() throws Exception {
    for (String name :
        new String[] {
          "EnrichmentException",
          "ForwardProxyEnrichmentOptions",
          "ForwardProxyUserEnrichmentProvider",
          "StatsigFPUserBuilder"
        }) {
      assertTrue(
          Class.forName("com.statsig.experimental." + name)
              .getPackage()
              .getName()
              .equals("com.statsig.experimental"));
      assertThrows(ClassNotFoundException.class, () -> Class.forName("com.statsig." + name));
    }
  }

  @BeforeEach
  public void setUp() throws Exception {
    mockWebServer = new MockWebServer();
    mockWebServer.start();
  }

  @AfterEach
  public void tearDown() throws Exception {
    mockWebServer.shutdown();
  }

  @Test
  public void testEmptyMembershipSuccess() {
    mockWebServer.enqueue(
        new MockResponse()
            .setResponseCode(200)
            .setHeader("Content-Type", "application/json")
            .setBody("{\"id_lists\":{}}"));

    ForwardProxyEnrichmentOptions options =
        new ForwardProxyEnrichmentOptions(mockWebServer.url("/").toString());
    options.timeoutMs = 500L;
    options.retries = 0;
    ForwardProxyUserEnrichmentProvider provider =
        new ForwardProxyUserEnrichmentProvider("secret-test", options);
    StatsigUser user = new StatsigFPUserBuilder(provider).setUserID("user-1").buildBlocking();
    try {
      assertTrue(user.getRef() != 0);
      assertEquals("user-1", user.getUserID());
      assertNull(user.getEmail());
      assertNull(user.getCustomIDs());
    } finally {
      user.close();
    }
  }

  @Test
  public void testBlockingBuildPreservesAllFields() {
    enqueueSuccess();
    StatsigUser user = fullBuilder().buildBlocking();
    try {
      assertAllFields(user);
      assertEquals(1, mockWebServer.getRequestCount());
    } finally {
      user.close();
    }
  }

  @Test
  public void testAsyncBuildPreservesAllFields() throws Exception {
    enqueueSuccess();
    StatsigUser user = fullBuilder().build().get(5, TimeUnit.SECONDS);
    try {
      assertAllFields(user);
      assertEquals(1, mockWebServer.getRequestCount());
    } finally {
      user.close();
    }
  }

  @Test
  public void testAsyncBuildKeepsFieldsFromBeforeEnrichment() throws Exception {
    mockWebServer.enqueue(
        new MockResponse()
            .setHeader("Content-Type", "application/json")
            .setBody("{\"id_lists\":{}}")
            .setBodyDelay(200, TimeUnit.MILLISECONDS));
    StatsigFPUserBuilder builder = fullBuilder();
    CompletableFuture<StatsigUser> pending = builder.build();
    assertNotNull(mockWebServer.takeRequest(5, TimeUnit.SECONDS));
    builder
        .setUserID("next-user")
        .setEmail("next@example.com")
        .setCustomIDs(Collections.singletonMap("companyID", "next-company"));
    StatsigUser user = pending.get(5, TimeUnit.SECONDS);
    try {
      assertAllFields(user);
    } finally {
      user.close();
    }
  }

  @Test
  public void testCustomIdOnlyPreservesMissingUserId() {
    enqueueSuccess();
    StatsigUser user =
        new StatsigFPUserBuilder(provider())
            .setCustomIDs(Collections.singletonMap("companyID", "company-1"))
            .buildBlocking();
    try {
      assertNull(user.getUserID());
      assertEquals(Collections.singletonMap("companyID", "company-1"), user.getCustomIDs());
    } finally {
      user.close();
    }
  }

  private void enqueueSuccess() {
    mockWebServer.enqueue(
        new MockResponse()
            .setHeader("Content-Type", "application/json")
            .setBody("{\"id_lists\":{}}"));
  }

  private ForwardProxyUserEnrichmentProvider provider() {
    ForwardProxyEnrichmentOptions options =
        new ForwardProxyEnrichmentOptions(mockWebServer.url("/").toString());
    options.timeoutMs = 2000L;
    options.retries = 0;
    return new ForwardProxyUserEnrichmentProvider("secret-test", options);
  }

  private Map<String, Object> customFields() {
    Map<String, Object> custom = new HashMap<>();
    custom.put("plan", "gold");
    custom.put("seats", 5);
    custom.put("enabled", true);
    custom.put("tags", Arrays.asList("alpha", "beta"));
    custom.put("nested", Collections.singletonMap("region", "west"));
    return custom;
  }

  private StatsigFPUserBuilder fullBuilder() {
    return new StatsigFPUserBuilder(provider())
        .setUserID("user-1")
        .setCustomIDs(Collections.singletonMap("companyID", "company-1"))
        .setEmail("test@example.com")
        .setIp("1.2.3.4")
        .setUserAgent("Kong FP test")
        .setCountry("US")
        .setLocale("en_US")
        .setAppVersion("1.2.3")
        .setCustom(customFields())
        .setPrivateAttributes(Collections.singletonMap("internal", "yes"));
  }

  private void assertAllFields(StatsigUser user) {
    assertTrue(user.getRef() != 0);
    assertEquals(StatsigUser.class, user.getClass());
    assertEquals("user-1", user.getUserID());
    assertEquals(Collections.singletonMap("companyID", "company-1"), user.getCustomIDs());
    assertEquals("test@example.com", user.getEmail());
    assertEquals("1.2.3.4", user.getIp());
    assertEquals("Kong FP test", user.getUserAgent());
    assertEquals("US", user.getCountry());
    assertEquals("en_US", user.getLocale());
    assertEquals("1.2.3", user.getAppVersion());
    assertEquals(customFields(), user.getCustom());
    assertEquals(Collections.singletonMap("internal", "yes"), user.getPrivateAttributes());
  }

  @Test
  public void testHttpFailure() {
    mockWebServer.enqueue(new MockResponse().setResponseCode(503).setBody("nope"));

    ForwardProxyEnrichmentOptions options =
        new ForwardProxyEnrichmentOptions(mockWebServer.url("/").toString());
    options.timeoutMs = 500L;
    options.retries = 0;
    ForwardProxyUserEnrichmentProvider provider =
        new ForwardProxyUserEnrichmentProvider("secret-test", options);
    EnrichmentException error =
        assertThrows(
            EnrichmentException.class,
            () -> new StatsigFPUserBuilder(provider).setUserID("user-1").buildBlocking());
    assertTrue(error.getMessage().contains("HTTP 503"));
  }
}
