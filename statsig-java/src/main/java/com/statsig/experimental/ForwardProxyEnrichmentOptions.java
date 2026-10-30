package com.statsig.experimental;

import com.statsig.ProxyConfig;

/**
 * <b>Experimental.</b> Options for {@link ForwardProxyUserEnrichmentProvider}. This API is unstable
 * and may change or be removed without notice.
 */
public class ForwardProxyEnrichmentOptions {
  public String url;
  public Long timeoutMs;
  public Integer retries;
  public ProxyConfig proxyConfig;

  public ForwardProxyEnrichmentOptions(String url) {
    this.url = url;
  }
}
