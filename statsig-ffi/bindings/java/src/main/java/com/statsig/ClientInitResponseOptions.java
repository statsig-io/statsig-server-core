package com.statsig;

public class ClientInitResponseOptions {
    public HashAlgo hashAlgo;
    private String hashAlgoInternal; // jni use string type

    public ClientInitResponseOptions(HashAlgo hashAlgo) {
        this.hashAlgo = hashAlgo;
        hashAlgoInternal = hashAlgo.convertToStr();
    }

    public ClientInitResponseOptions() {
        this.hashAlgo = HashAlgo.SHA256;
    }

    public HashAlgo getHashAlgo() {
        return hashAlgo;
    }

    public void setHashAlgo(HashAlgo hashAlgo) {
        this.hashAlgo = hashAlgo;
        hashAlgoInternal = hashAlgo.convertToStr();
    }
}
