package com.statsig;

public enum HashAlgo {
    DJB2,
    SHA256,
    NONE;

    String convertToStr() {
        switch (this) {
            case DJB2: return "djb2";
            case SHA256: return "sha256";
            default: return "none";
        }
    }
}
