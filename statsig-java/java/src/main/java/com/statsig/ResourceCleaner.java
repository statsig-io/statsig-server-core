package com.statsig;

import java.lang.ref.Cleaner;

class ResourceCleaner {
    private static final Cleaner CLEANER = Cleaner.create();

    static void register(Object object, Runnable cleanupTask) {
        CLEANER.register(object, cleanupTask);
    }
}
