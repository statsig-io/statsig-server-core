package com.statsig.experimental;

import com.statsig.ExperimentalUserEnrichmentBridge;

/**
 * <b>Experimental.</b> Resolves ID-list memberships via a Statsig forward proxy. Share one instance
 * across {@link StatsigFPUserBuilder}s. This API is unstable and may change or be removed without
 * notice.
 */
public class ForwardProxyUserEnrichmentProvider {
  private volatile long ref;

  public ForwardProxyUserEnrichmentProvider(String sdkKey, ForwardProxyEnrichmentOptions options) {
    long timeout = options.timeoutMs == null ? -1 : options.timeoutMs;
    long retries = options.retries == null ? -1 : options.retries;
    this.ref =
        ExperimentalUserEnrichmentBridge.createProvider(
            this, sdkKey, options.url, timeout, retries, options.proxyConfig);
    if (this.ref == 0) {
      throw new EnrichmentException("Failed to create ForwardProxyUserEnrichmentProvider");
    }
  }

  long getRef() {
    return ref;
  }
}
