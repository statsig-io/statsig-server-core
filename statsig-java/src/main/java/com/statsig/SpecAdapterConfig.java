package com.statsig;

public class SpecAdapterConfig {
  private String adapterType;
  private String specsUrl;
  private long initTimeoutMs;

  private String authenticationMode;
  private String caCertPath;
  private String clientCertPath;
  private String clientKeyPath;
  private String domainName;

  public String getAdapterType() {
    return adapterType;
  }

  public SpecAdapterConfig setAdapterType(String adapterType) {
    this.adapterType = adapterType;
    return this;
  }

  public String getSpecsUrl() {
    return specsUrl;
  }

  public SpecAdapterConfig setSpecsUrl(String specsUrl) {
    this.specsUrl = specsUrl;
    return this;
  }

  public long getInitTimeoutMs() {
    return initTimeoutMs;
  }

  public SpecAdapterConfig setInitTimeoutMs(long initTimeoutMs) {
    this.initTimeoutMs = initTimeoutMs;
    return this;
  }

  public String getAuthenticationMode() {
    return authenticationMode;
  }

  public SpecAdapterConfig setAuthenticationMode(String authenticationMode) {
    this.authenticationMode = authenticationMode;
    return this;
  }

  public String getCaCertPath() {
    return caCertPath;
  }

  public SpecAdapterConfig setCaCertPath(String caCertPath) {
    this.caCertPath = caCertPath;
    return this;
  }

  public String getClientCertPath() {
    return clientCertPath;
  }

  public SpecAdapterConfig setClientCertPath(String clientCertPath) {
    this.clientCertPath = clientCertPath;
    return this;
  }

  public String getClientKeyPath() {
    return clientKeyPath;
  }

  public SpecAdapterConfig setClientKeyPath(String clientKeyPath) {
    this.clientKeyPath = clientKeyPath;
    return this;
  }

  public String getDomainName() {
    return domainName;
  }

  public SpecAdapterConfig setDomainName(String domainName) {
    this.domainName = domainName;
    return this;
  }
}
