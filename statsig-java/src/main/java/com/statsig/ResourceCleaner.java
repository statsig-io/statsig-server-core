package com.statsig;

import java.lang.ref.PhantomReference;
import java.lang.ref.ReferenceQueue;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

class ResourceCleaner {
  private static final String TAG = "StatsigResourceCleaner";
  private static final boolean IS_JAVA_8 = isJava8();

  // --- Java 8 Cleaner ---
  private static final ReferenceQueue<Object> QUEUE = new ReferenceQueue<>();
  private static final Map<PhantomReference<?>, Runnable> CLEAN_TASKS = new ConcurrentHashMap<>();

  static {
    if (IS_JAVA_8) {
      Thread cleanerThread =
          new Thread(
              () -> {
                while (true) {
                  try {
                    PhantomReference<?> ref = (PhantomReference<?>) QUEUE.remove();
                    Runnable cleanTask = CLEAN_TASKS.remove(ref);

                    if (cleanTask != null) {
                      cleanTask.run();
                    }
                  } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                    OutputLogger.logError(TAG, "Cleaner thread interrupted. Exiting...");
                    break;
                  } catch (Throwable t) {
                    OutputLogger.logError(TAG, "Unexpected error in cleaner thread: " + t);
                  }
                }
              });
      cleanerThread.setDaemon(true);
      cleanerThread.start();
    }
  }

  // --- Public Usage API ---
  // NOTE: The usage is unified across Java versions — callers do not need to worry
  // about the underlying mechanism.
  public static Cleanable register(Object obj, Runnable cleanupTask) {
    if (IS_JAVA_8) {
      PhantomCleanable ref = new PhantomCleanable(obj, QUEUE, cleanupTask);
      CLEAN_TASKS.put(ref, cleanupTask);
      return ref;
    } else {
      // For Java 9 and above:
      // We delegate to the Java9CleanerHolder which internally uses java.lang.ref.Cleaner.
      // This block is only executed if Java version is not 1.8, and Cleaner is loaded lazily
      // to avoid classloading errors on Java 8.
      return Java9CleanerHolder.register(obj, cleanupTask);
    }
  }

  public interface Cleanable {
    void clean();
  }

  private static boolean isJava8() {
    String version = System.getProperty("java.version");
    return version.startsWith("1.8");
  }

  // --- For Java 8 use case ---
  private static class PhantomCleanable extends PhantomReference<Object> implements Cleanable {
    private Runnable task;

    PhantomCleanable(Object referent, ReferenceQueue<? super Object> queue, Runnable task) {
      super(referent, queue);
      this.task = task;
    }

    @Override
    public void clean() {
      Runnable t = CLEAN_TASKS.remove(this);
      if (t != null) {
        t.run();
      }
      this.task = null;
    }
  }

  // --- Java 9 Cleaner (inner class to avoid classloading issues on Java 8) ---
  private static class Java9CleanerHolder {
    private static final Object cleaner = createCleaner();

    private static Object createCleaner() {
      try {
        Class<?> cleanerClass = Class.forName("java.lang.ref.Cleaner");
        return cleanerClass.getMethod("create").invoke(null);
      } catch (Throwable t) {
        OutputLogger.logError(TAG, "Failed to initialize Java 9 Cleaner" + t);
        return null;
      }
    }

    static Cleanable register(Object obj, Runnable task) {
      if (cleaner == null) {
        OutputLogger.logError(TAG, "Cleaner not available. Cleanup task will not be registered.");
        return () -> {}; // No-op fallback
      }

      try {
        Class<?> cleanerClass = Class.forName("java.lang.ref.Cleaner");
        Object cleanable =
            cleanerClass
                .getMethod("register", Object.class, Runnable.class)
                .invoke(cleaner, obj, task);

        return () -> {
          try {
            cleanable.getClass().getMethod("clean").invoke(cleanable);
          } catch (Throwable ignored) {
          }
        };
      } catch (Throwable t) {
        OutputLogger.logError(TAG, "Failed to register cleanup task: " + t);
        return () -> {};
      }
    }
  }
}
