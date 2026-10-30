package com.statsig;

import org.jetbrains.annotations.ApiStatus;

/**
 * Internal access to package-private native resources for {@code com.statsig.experimental}. SDK
 * consumers should use the experimental package rather than this implementation helper.
 */
@ApiStatus.Internal
public final class ExperimentalUserEnrichmentBridge {
  private ExperimentalUserEnrichmentBridge() {}

  public static long createProvider(
      Object owner,
      String sdkKey,
      String url,
      long timeout,
      long retries,
      ProxyConfig proxyConfig) {
    long ref =
        StatsigJNI.statsigFpEnrichmentProviderCreate(sdkKey, url, timeout, retries, proxyConfig);
    if (ref != 0) {
      ResourceCleaner.register(owner, () -> StatsigJNI.statsigFpEnrichmentProviderRelease(ref));
    }
    return ref;
  }

  public static String buildBlocking(long providerRef, String userData) {
    return StatsigJNI.statsigFpUserBuilderBuildBlocking(providerRef, userData);
  }

  public static java.util.concurrent.CompletableFuture<StatsigUser> buildAsync(
      long providerRef, String jsonData, StatsigUser.Builder userData) {
    java.util.concurrent.CompletableFuture<StatsigUser> future =
        new java.util.concurrent.CompletableFuture<>();
    boolean started =
        StatsigJNI.statsigFpUserBuilderBuildAsync(
            providerRef,
            jsonData,
            resultJson -> {
              try {
                com.alibaba.fastjson2.JSONObject result =
                    com.alibaba.fastjson2.JSON.parseObject(resultJson);
                if (result.containsKey("error")) {
                  future.completeExceptionally(
                      new com.statsig.experimental.EnrichmentException(result.getString("error")));
                  return;
                }
                StatsigUser user = createUser(result.getLongValue("userRef"), userData);
                if (!future.complete(user)) {
                  user.close();
                }
              } catch (Exception error) {
                future.completeExceptionally(error);
              }
            });
    if (!started) {
      future.completeExceptionally(
          new com.statsig.experimental.EnrichmentException("Failed to start user enrichment"));
    }
    return future;
  }

  public static StatsigUser createUser(long userRef, StatsigUser.Builder userData) {
    return new StatsigUser(userRef, userData);
  }
}
