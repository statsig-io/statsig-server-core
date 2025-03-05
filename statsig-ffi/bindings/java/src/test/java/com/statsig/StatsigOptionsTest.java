package com.statsig;

import org.junit.jupiter.api.Test;

public class StatsigOptionsTest {
    @Test
    public void testStatsigOptions() {
        StatsigOptions options = new StatsigOptions.Builder()
                .setDisableAllLogging(true)
                .setEnableCountryLookup(true)
                .build();
        CheckGateOptions checkGateOptions = new CheckGateOptions(false);
    }
}
