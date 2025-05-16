package com.statsig;

import org.jetbrains.annotations.Nullable;

public class ClientInitResponseOptions {
  public HashAlgo hashAlgo;
  public String clientSDKKey;
  public boolean includeLocalOverrides;
  public GCIRResponseFormat responseFormat = GCIRResponseFormat.Initialize;
  private String hashAlgoInternal; // jni use string type
  private String responseFormatInternal; // jni use string type

  public ClientInitResponseOptions(
      @Nullable HashAlgo hashAlgo, String clientSDKKey, boolean includeLocalOverrides) {
    this.hashAlgo = hashAlgo;
    this.clientSDKKey = clientSDKKey;
    this.includeLocalOverrides = includeLocalOverrides;
    if (hashAlgo != null) {
      this.hashAlgoInternal = hashAlgo.convertToStr();
    }
  }

  public ClientInitResponseOptions(String clientSDKKey) {
    this(null, clientSDKKey, false);
  }

  public ClientInitResponseOptions(HashAlgo hashAlgo) {
    this(hashAlgo, null, false);
  }

  public ClientInitResponseOptions(HashAlgo hashAlgo, String clientSDKKey) {
    this(hashAlgo, clientSDKKey, false);
  }

  public ClientInitResponseOptions(HashAlgo hashAlgo, boolean includeLocalOverrides) {
    this(hashAlgo, null, includeLocalOverrides);
  }

  public ClientInitResponseOptions() {
    this(null, null, false);
  }

  public HashAlgo getHashAlgo() {
    return hashAlgo;
  }

  public void setHashAlgo(HashAlgo hashAlgo) {
    this.hashAlgo = hashAlgo;
    this.hashAlgoInternal = (hashAlgo != null) ? hashAlgo.convertToStr() : null;
  }

  public void setClientSDKKey(String clientSDKKey) {
    this.clientSDKKey = clientSDKKey;
  }

  public void setIncludeLocalOverrides(boolean includeLocalOverrides) {
    this.includeLocalOverrides = includeLocalOverrides;
  }

  public void setResponseFormat(GCIRResponseFormat responseFormat) {
    this.responseFormat = responseFormat;
    responseFormatInternal = responseFormat.convertToStr();
  }
}
