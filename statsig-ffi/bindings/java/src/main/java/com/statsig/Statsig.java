package com.statsig;

import com.google.gson.Gson;
import com.statsig.internal.GsonUtil;

import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;

public class Statsig {
    private static final Gson gson = GsonUtil.getGson();

    private volatile String ref;
    private final ScheduledExecutorService scheduler = Executors.newScheduledThreadPool(1);

    /**
     * Instantiates a new Statsig instance that connects to Statsig Service.
     * <p>
     * It is recommended to create a single instance for the entire application's
     * lifecycle. In rare situations where
     * the application requires feature evaluation from different Statsig projects
     * or environments, you may instantiate
     * multiple instances. However, these should be maintained throughout the
     * application's lifecycle, rather than
     * being created for each request or thread.
     *
     * @param sdkKey  secret key to connect to Statsig Service
     * @param options a customized instance of StatsigOptions that configures the
     *                behavior of the
     *                Statsig instance.
     */
    public Statsig(String sdkKey, StatsigOptions options) {
        this.ref = StatsigJNI.statsigCreate(sdkKey, options.getRef());

        ResourceCleaner.register(this, () -> {
            if (ref != null) {
                StatsigJNI.statsigRelease(ref);
                ref = null;
            }
        });
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

    public CompletableFuture<Void> shutdown() {
        CompletableFuture<Void> future = new CompletableFuture<>();
        Runnable callback = () -> {
            scheduler.execute(() -> {
                StatsigJNI.statsigFinalizeShutdown(ref);
                future.complete(null);
                scheduler.shutdown();
            });
        };

        StatsigJNI.statsigSequencedShutdownPrepare(ref, callback);

        return future;
    }

    public boolean checkGate(StatsigUser user, String gateName) {
        return StatsigJNI.statsigCheckGate(ref, user.getRef(), gateName, null);
    }

    public boolean checkGate(StatsigUser user, String gateName, CheckGateOptions options) {
        return StatsigJNI.statsigCheckGate(ref, user.getRef(), gateName, options);
    }

    public void manuallyLogGateExposure(StatsigUser user, String gateName) {
        StatsigJNI.statsigLogGateExposure(ref, user.getRef(), gateName);
    }

    public Experiment getExperiment(StatsigUser user, String experimentName) {
        String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName, null);
        Experiment experiment = gson.fromJson(experJson, Experiment.class);
        if (experiment != null) {
            experiment.setRawJson(experJson);
        }
        return experiment;
    }

    public Experiment getExperiment(StatsigUser user, String experimentName, GetExperimentOptions options) {
        String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName, options);
        Experiment experiment = gson.fromJson(experJson, Experiment.class);
        if (experiment != null) {
            experiment.setRawJson(experJson);
        }
        return experiment;
    }

    public void manuallyLogExperimentExposure(StatsigUser user, String experimentName) {
        StatsigJNI.statsigLogExperimentExposure(ref, user.getRef(), experimentName);
    }

    public DynamicConfig getDynamicConfig(StatsigUser user, String configName) {
        String configJson = StatsigJNI.statsigGetDynamicConfig(ref, user.getRef(), configName, null);
        DynamicConfig dynamicConfig = gson.fromJson(configJson, DynamicConfig.class);
        if (dynamicConfig != null) {
            dynamicConfig.setRawJson(configJson);
        }
        return dynamicConfig;
    }

    public DynamicConfig getDynamicConfig(StatsigUser user, String configName, GetDynamicConfigOptions options) {
        String configJson = StatsigJNI.statsigGetDynamicConfig(ref, user.getRef(), configName, options);
        DynamicConfig dynamicConfig = gson.fromJson(configJson, DynamicConfig.class);
        if (dynamicConfig != null) {
            dynamicConfig.setRawJson(configJson);
        }
        return dynamicConfig;
    }

    public void manuallyLogDynamicConfigExposure(StatsigUser user, String configName) {
        StatsigJNI.statsigLogDynamicConfigExposure(ref, user.getRef(), configName);
    }

    public Layer getLayer(StatsigUser user, String layerName) {
        String layerJson = StatsigJNI.statsigGetLayer(ref, user.getRef(), layerName, null);
        Layer layer = gson.fromJson(layerJson, Layer.class);
        if (layer != null) {
            // Set the Statsig reference in the Layer instance
            layer.setStatsigInstance(this);
            layer.setRawJson(layerJson);
        }
        return layer;
    }

    public Layer getLayer(StatsigUser user, String layerName, GetLayerOptions options) {
        String layerJson = StatsigJNI.statsigGetLayer(ref, user.getRef(), layerName, options);
        Layer layer = gson.fromJson(layerJson, Layer.class);
        if (layer != null) {
            // Set the Statsig reference in the Layer instance
            layer.setStatsigInstance(this);
            layer.setRawJson(layerJson);
            layer.setDisableExposureLogging(options != null && options.disableExposureLogging);
        }
        return layer;
    }

    public void manuallyLogLayerParamExposure(StatsigUser user, String layerName, String param) {
        StatsigJNI.statsigManuallyLogLayerParamExposure(ref, user.getRef(), layerName, param);
    }

    public FeatureGate getFeatureGate(StatsigUser user, String gateName) {
        String gateJson = StatsigJNI.statsigGetFeatureGate(ref, user.getRef(), gateName, null);
        FeatureGate featureGate = gson.fromJson(gateJson, FeatureGate.class);
        if (featureGate != null) {
            featureGate.setRawJson(gateJson);
        }
        return featureGate;
    }

    public FeatureGate getFeatureGate(StatsigUser user, String gateName, CheckGateOptions options) {
        String gateJson = StatsigJNI.statsigGetFeatureGate(ref, user.getRef(), gateName, options);
        FeatureGate featureGate = gson.fromJson(gateJson, FeatureGate.class);
        if (featureGate != null) {
            featureGate.setRawJson(gateJson);
        }
        return featureGate;
    }

    public CMABRankedVariant[] getCMABRankedVariants(StatsigUser user, String cmabName) {
        String cmabJson = StatsigJNI.statsigGetCMABRankedVariants(ref, user.getRef(), cmabName);
        CMABRankedVariant[] cmabRankedVariants = gson.fromJson(cmabJson, CMABRankedVariant[].class);
        return cmabRankedVariants;
    }

    public void logCMABExposure(StatsigUser user, String cmabName, String ruleId) {
        StatsigJNI.statsigLogCMABExposure(ref, user.getRef(), cmabName, ruleId);
    }

    public void logEvent(StatsigUser user, String eventName, String value, Map<String, String> metadata) {
        StatsigJNI.statsigLogEvent(ref, user.getRef(), eventName, value, metadata);
    }

    public void logEvent(StatsigUser user, String eventName, long value, Map<String, String> metadata) {
        StatsigJNI.statsigLogEventWithLong(ref, user.getRef(), eventName, value, metadata);
    }

    public void logEvent(StatsigUser user, String eventName, double value, Map<String, String> metadata) {
        StatsigJNI.statsigLogEventWithDouble(ref, user.getRef(), eventName, value, metadata);
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
        // if no gcir option passed in, will default to djb2
        return StatsigJNI.statsigGetClientInitResponse(ref, user.getRef(), null);
    }

    public String getClientInitializeResponse(StatsigUser user, ClientInitResponseOptions options) {
        return StatsigJNI.statsigGetClientInitResponse(ref, user.getRef(), options);
    }

    void logLayerParamExposure(String layerJson, String param) {
        StatsigJNI.statsigLogLayerParamExposure(ref, layerJson, param);
    }
}
