package com.statsig;

import com.google.gson.Gson;
import com.statsig.internal.GsonUtil;

import java.util.Map;
import java.util.concurrent.CompletableFuture;

public class Statsig implements AutoCloseable {
    private static final Gson gson = GsonUtil.getGson();

    private volatile String ref;

    /**
     * Instantiates a new Statsig instance that connects to Statsig Service.
     * <p>
     * It is recommended to create a single instance for the entire application's lifecycle. In rare situations where
     * the application requires feature evaluation from different Statsig projects or environments, you may instantiate
     * multiple instances. However, these should be maintained throughout the application's lifecycle, rather than
     * being created for each request or thread.
     *
     * @param sdkKey  secret key to connect to Statsig Service
     * @param options a customized instance of StatsigOptions that configures the behavior of the
     *                Statsig instance.
     */
    public Statsig(String sdkKey, StatsigOptions options) {
        this.ref = StatsigJNI.statsigCreate(sdkKey, options.getRef());
    }

    public String getRef() {
        return ref;
    }

    public CompletableFuture<Void> initialize() {
        CompletableFuture<Void> future = new CompletableFuture<>();
        Runnable callback = () -> {
            // Complete the future when the native operation is done
            future.complete(null);
        };

        StatsigJNI.statsigInitialize(ref, callback);
        return future;
    }

    public boolean checkGate(StatsigUser user, String gateName) {
        return StatsigJNI.statsigCheckGate(ref, user.getRef(), gateName);
    }

    public Experiment getExperiment(StatsigUser user, String experimentName) {
        String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName);
        return gson.fromJson(experJson, Experiment.class);
    }

    public DynamicConfig getDynamicConfig(StatsigUser user, String configName) {
        String configJson = StatsigJNI.statsigGetDynamicConfig(ref, user.getRef(), configName);
        return gson.fromJson(configJson, DynamicConfig.class);
    }

    public Layer getLayer(StatsigUser user, String layerName) {
        String layerJson = StatsigJNI.statsigGetLayer(ref, user.getRef(), layerName);
        return gson.fromJson(layerJson, Layer.class);
    }

    public FeatureGate getFeatureGate(StatsigUser user, String gateName) {
        String gateJson = StatsigJNI.statsigGetFeatureGate(ref, user.getRef(), gateName);
        FeatureGate featureGate = gson.fromJson(gateJson, FeatureGate.class);
        featureGate.setRawJson(gateJson);
        return featureGate;
    }

    public void logEvent(StatsigUser user, String eventName, String value, Map<String, String> metadata) {
        StatsigJNI.statsigLogEvent(ref, user.getRef(), eventName, value, metadata);
    }

    public CompletableFuture<Void> flushEvents() {
        CompletableFuture<Void> future = new CompletableFuture<>();
        Runnable callback = () -> {
            future.complete(null);
        };

        StatsigJNI.statsigFlushEvents(ref, callback);
        return future;
    }

    public String getClientInitializeResponse(StatsigUser user) {
        return StatsigJNI.statsigGetClientInitResponse(ref, user.getRef());
    }

    public CompletableFuture<Void> shutdown() {
        CompletableFuture<Void> future = new CompletableFuture<>();
        Runnable callback = () -> {
            // Complete the future when the native operation is done
            future.complete(null);
        };

        StatsigJNI.statsigShutdown(ref, callback);
        this.close();
        return future;
    }

    @Override
    public synchronized void close() {
        if (ref != null) {
            StatsigJNI.statsigRelease(ref);
            this.ref = null;
        }
    }
}
