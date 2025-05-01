package com.statsig;

public enum GCIRResponseFormat {
    Initialize,
    InitializeWithSecondaryExposureMapping;

    String convertToStr() {
        switch (this) {
            case Initialize:
                return "v1";
            case InitializeWithSecondaryExposureMapping:
                return "v2";
            default:
                return "v1";
        }
    }
}
