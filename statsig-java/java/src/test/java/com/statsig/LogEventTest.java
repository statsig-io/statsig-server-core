package com.statsig;

import java.util.HashMap;
import java.util.concurrent.ExecutionException;

import org.junit.Test;

public class LogEventTest {
    @Test
    public void testAllAPIs() throws InterruptedException, ExecutionException {
        Statsig statsig = new Statsig("secret-key");
        statsig.initialize().get();
        StatsigUser user = new StatsigUser.Builder().setUserID("userID").build();
        HashMap<String, String> metadata = new HashMap<String,String>();
        metadata.put("key1", "value");
        statsig.logEvent(user, "custom_event");
        statsig.logEvent(user, "custom_event", 12.2);
        statsig.logEvent(user, "custom_event", 12.2, metadata);
        statsig.logEvent(user, "custom_event", "value");
        statsig.logEvent(user, "custom_event", "value", metadata);
    }
}
