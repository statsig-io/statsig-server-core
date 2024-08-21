package com.statsig;

import com.google.gson.Gson;

import java.nio.ByteBuffer;
import java.util.concurrent.CompletableFuture;

public class Statsig implements AutoCloseable {
    private final long statsigRef;
    private final ByteBuffer buffer;
    private final Gson gson;

    /**
     * Instantiates a new Statsig instance that connects to Statsig Service.
     * <p>
     * It is recommended to create a single instance for the entire application's lifecycle. In rare situations where
     * the application requires feature evaluation from different Statsig projects or environments, you may instantiate
     * multiple instances. However, these should be maintained throughout the application's lifecycle, rather than
     * being created for each request or thread.
     *
     * @param sdkKey secret key to connect to Statsig Service
     * @param options a customized instance of StatsigOptions that configures the behavior of the
     *            Statsig instance.
     */
    public Statsig(String sdkKey, StatsigOptions options) {
        int estimatedSize = 1024 * 1024 * 5;
        this.buffer = ByteBuffer.allocateDirect(estimatedSize);
        this.statsigRef = StatsigJNI.statsigCreate(sdkKey, options.getRef());
        this.gson = new Gson();
    }

    public long getRef() {
        return statsigRef;
    }

    public CompletableFuture<Void> initialize() {
        CompletableFuture<Void> future = new CompletableFuture<>();
        Runnable callback = () -> {
            // Complete the future when the native operation is done
            future.complete(null);
        };

        StatsigJNI.statsigInitialize(statsigRef, callback);
        return future;
    }

    public boolean checkGate(StatsigUser user, String gateName) {
        return StatsigJNI.statsigCheckGate(statsigRef, user.getRef(), gateName);
    }

    public String getClientInitializeResponse(StatsigUser user) {
        return StatsigJNI.statsigGetClientInitResponse(statsigRef, user.getRef());
    }

    @Override
    public void close() {
        if (statsigRef != 0) {
            StatsigJNI.statsigRelease(statsigRef);
        }
    }
}
