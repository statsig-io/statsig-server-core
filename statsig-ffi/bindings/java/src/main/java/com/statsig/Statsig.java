package com.statsig;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;

import com.statsig.internal.GsonUtil;

import java.lang.reflect.Type;
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

    public Statsig(String sdkKey) {
        this.ref = StatsigJNI.statsigCreate(sdkKey, null);

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

    public String[] getFieldsNeededForGate(String gateName) {
        String resultJSON = StatsigJNI.statsigGetFieldsNeededForGate(ref, gateName);
        return resultJSON == null ? new String[0] : gson.fromJson(resultJSON, String[].class);
    }

    public Experiment getExperiment(StatsigUser user, String experimentName) {
        String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName, null);
        return Experiment.fromJson(experJson);
    }

    public Experiment getExperiment(StatsigUser user, String experimentName, GetExperimentOptions options) {
        String experJson = StatsigJNI.statsigGetExperiment(ref, user.getRef(), experimentName, options);
        return Experiment.fromJson(experJson);
    }

    public void manuallyLogExperimentExposure(StatsigUser user, String experimentName) {
        StatsigJNI.statsigLogExperimentExposure(ref, user.getRef(), experimentName);
    }

    public String[] getFieldsNeededForExperiment(String experimentName) {
        String resultJSON = StatsigJNI.statsigGetFieldsNeededForExperiment(ref, experimentName);
        return resultJSON == null ? new String[0] : gson.fromJson(resultJSON, String[].class);
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

    public String[] getFieldsNeededForDynamicConfig(String configName) {
        String resultJSON = StatsigJNI.statsigGetFieldsNeededForDynamicConfig(ref, configName);
        return resultJSON == null ? new String[0] : gson.fromJson(resultJSON, String[].class);
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

    public String[] getFieldsNeededForLayer(String layerName) {
        String resultJSON = StatsigJNI.statsigGetFieldsNeededForLayer(ref, layerName);
        return resultJSON == null ? new String[0] : gson.fromJson(resultJSON, String[].class);
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

    public ParameterStore getParameterStore(StatsigUser user, String parameterStoreName) {
        String storeJson = StatsigJNI.statsigGetParameterStore(ref, parameterStoreName);
        ParameterStore store = gson.fromJson(storeJson, ParameterStore.class);
        if (store != null) {
            // Set the Statsig reference in the Layer instance
            store.setStatsigInstance(this);
            store.setUser(user);
        }
        return store;
    }

    public String getStringFromParameterStore(StatsigUser user, String parameterStoreName, String parameterName,
            String defaultValue) {
        return StatsigJNI.statsigGetStringParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValue);
    }

    public boolean getBooleanFromParameterStore(StatsigUser user, String parameterStoreName, String parameterName,
            boolean defaultValue) {
        return StatsigJNI.statsigGetBooleanParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValue);
    }

    public double getDoubleFromParameterStore(StatsigUser user, String parameterStoreName, String parameterName,
            double defaultValue) {
        return StatsigJNI.statsigGetFloatParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValue);
    }

    public long getLongFromParameterStore(StatsigUser user, String parameterStoreName, String parameterName,
            long defaultValue) {
        return StatsigJNI.statsigGetIntegerParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValue);
    }

    public int getIntFromParameterStore(StatsigUser user, String parameterStoreName, String parameterName,
            int defaultValue) {
        return (int) StatsigJNI.statsigGetIntegerParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValue);
    }

    public Map<String, Object> getMapFromParameterStore(StatsigUser user, String parameterStoreName,
            String parameterName, Map<String, Object> defaultValue) {
        String defaultValueJSON = defaultValue == null ? null : gson.toJson(defaultValue);
        String result = StatsigJNI.statsigGetObjectParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValueJSON);
        if (result == null) {
            return defaultValue;
        }
        Type mapType = new TypeToken<Map<String, Object>>() {
        }.getType();
        Map<String, Object> map = gson.fromJson(result, mapType);
        if (map == null) {
            return defaultValue;
        }
        return map;
    }

    public Object[] getArrayFromParameterStore(StatsigUser user, String parameterStoreName, String parameterName,
            Object[] defaultValue) {
        String defaultValueJSON = defaultValue == null ? null : gson.toJson(defaultValue);
        String result = StatsigJNI.statsigGetArrayParameterFromParameterStore(ref, user.getRef(), parameterStoreName,
                parameterName, defaultValueJSON);
        Object[] array = gson.fromJson(result, Object[].class);
        if (array == null) {
            return defaultValue;
        }
        return array;
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
