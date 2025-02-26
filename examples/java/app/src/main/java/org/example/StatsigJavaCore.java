/*
 * Sample App For Using Statsig Java Core
 */
package org.example;


import com.statsig.*;

import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ExecutionException;

public class StatsigJavaCore {
    public static void main(String[] args) {
        /**
         * Customize statsig options as needed, e.g., options.setSpecsSyncIntervalMs(5000);
         */
        StatsigOptions options = new StatsigOptions.Builder()
                .setSpecsSyncIntervalMs(5000)
                .setOutputLoggerLevel(OutputLogger.LogLevel.ERROR)
                .build();

        Statsig statsig = new Statsig("secret-key", options);

        /**
         * initialize statsig server
         */
        try {
            statsig.initialize().get();
        } catch (InterruptedException | ExecutionException e) {
            System.err.println(e.getMessage());
        }

        // --------- Creating Statsig User -----------

        StatsigUser user = new StatsigUser.Builder()
                .setUserID("user_id")
                .setCustomIDs(Map.of("external_id", "abc123"))
                .setEmail("user@example.com")
                .setCountry("USA")
                .setPrivateAttributes(Map.of("is_beta_user", "true"))
                .setCustom(Map.of("subscription", "premium"))
                .build();

        // --------- Working With SDK -----------

        /**
         * Checking a gate
         */
        boolean isFeatureOn = statsig.checkGate(user, "example_gate");
        if (isFeatureOn) {
            System.out.println("Gate is on");
        } else {
            System.out.println("Gate is off");
        }

        /**
         * Getting a dynamic config
         */
        DynamicConfig config = statsig.getDynamicConfig(user, "awesome_product_details");

        /**
         * Getting a Layer/Experiment
         */
        Experiment experiment = statsig.getExperiment(user, "sample_exp");
        Layer layer = statsig.getLayer(user, "layer_name");

        /**
         * Now that you have a Feature Gate or an Experiment set up,
         * you may want to track some custom events and see how your new features or
         * different experiment groups affect these events.
         *
         * Logging an Event
         */
        String eventName = "purchase";
        String value = "100";
        Map<String, String> metadata = new HashMap<>();
        metadata.put("item_id", "12345");
        metadata.put("category", "electronics");

        statsig.logEvent(user, eventName, value, metadata);


        // --------- Shutdown Statsig SDK -----------
        try {
            statsig.shutdown().get();
            System.out.println("Statsig instance have been successfully shutdown.");
        } catch (InterruptedException | ExecutionException e) {
            System.err.println(e.getMessage());
        }
    }

}
