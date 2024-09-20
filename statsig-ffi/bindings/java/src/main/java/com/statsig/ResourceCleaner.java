package com.statsig;

import java.lang.ref.Cleaner;

class ResourceCleaner {
    private static final Cleaner cleaner = Cleaner.create();

    static void register(Object object, Runnable cleanupTask) {
        cleaner.register(object, cleanupTask);
    }
}
