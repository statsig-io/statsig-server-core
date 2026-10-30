package com.statsig.experimental;

/**
 * <b>Experimental.</b> Thrown when forward-proxy user enrichment fails (timeout, HTTP error,
 * transport, or decode failure). This API is unstable and may change or be removed without notice.
 */
public class EnrichmentException extends RuntimeException {
  public EnrichmentException(String message) {
    super(message);
  }
}
