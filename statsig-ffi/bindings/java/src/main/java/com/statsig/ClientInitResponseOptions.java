package com.statsig;

public class ClientInitResponseOptions {
    public HashAlgo hashAlgo;
    public String clientSDKKey;
    private String hashAlgoInternal; // jni use string type

    public ClientInitResponseOptions(HashAlgo hashAlgo, String clientSDKKey) {
        this.hashAlgo = hashAlgo;
        this.clientSDKKey = clientSDKKey;
        hashAlgoInternal = hashAlgo.convertToStr();
    }

    public HashAlgo getHashAlgo() {
        return hashAlgo;
    }

    public void setHashAlgo(HashAlgo hashAlgo) {
        this.hashAlgo = hashAlgo;
        hashAlgoInternal = hashAlgo.convertToStr();
    }

    public void setClientSDKKey(String clientSDKKey) {
        this.clientSDKKey = clientSDKKey;
    }
}
