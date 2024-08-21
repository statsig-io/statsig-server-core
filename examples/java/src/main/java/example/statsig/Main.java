package example.statsig;

import com.google.gson.Gson;
import com.statsig.StatsigJNI;

import java.util.*;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutionException;


public class Main {
    static List<String> getGates() {
        return Arrays.asList();
    }

    public static void main(String[] args) throws ExecutionException, InterruptedException {
        if (!StatsigJNI.isLibraryLoaded()) {
            System.out.println("Statsig library not loaded");
            return;
        }

        long statsigRef = StatsigJNI.statsigCreate("", -1);
        System.out.println("[Java] Statsig " + statsigRef);

        CountDownLatch latch = new CountDownLatch(1);

        Gson gson = new Gson();

        Map<String, String> customIDs  = new HashMap<>() {{
            put("podID", "scrapi-init-jpvgc");
        }};
        String customIDsJson = gson.toJson(customIDs);

        Map<String, String> custom  = new HashMap<>() {{
            put("flavor", "scrapi-init-cg");
            put("region", "gke-eu-west1");
            put("service", "scrapi-init");
            put("serviceVersion", "240814-61ec7ebf0395");
            put("tier", "prod");
        }};
        String customJson = gson.toJson(custom);

        long userRef = StatsigJNI.statsigUserCreate(
                null, customIDsJson, null, null, null, null, null,
                null, customJson, null
        );


        StatsigJNI.statsigInitialize(statsigRef, () -> {
            List<String> gates = getGates();
            long start = System.nanoTime();

            for (String gate : gates) {
                for (int j = 0; j < 1000; j++) {
                    boolean result = StatsigJNI.statsigCheckGate(statsigRef, userRef, gate);
                }
            }

            long end = System.nanoTime();

            System.out.println("all duration: " + (end - start) / 1_000_000.0 + " ms");


            StatsigJNI.statsigUserRelease(userRef);
            latch.countDown();
        });

        latch.await();

        StatsigJNI.statsigShutdown(statsigRef, () -> {
            System.out.println("[Java] Was Shutdown");
        });
        StatsigJNI.statsigRelease(statsigRef);

//        Perf Test


//        StatsigOptions options = new StatsigOptions();
//
////        MyLib lib = new MyLib();
//
//        Statsig statsig = new Statsig("", options);
//        statsig.initialize().get();
//
//        Map<String, Object> data = statsig.getCurrentValues();
//        Map<String, Object> values = (Map<String, Object>) data.get("values");
//        Map<String, Object> gates = (Map<String, Object>) values.get("feature_gates");
//
//        StatsigUser user = getUser();
//
//        Map<String, Double> times = new HashMap<>();
//        long allStart = System.nanoTime();
//
//        for (String gate : gates.keySet()) {
//            long start = System.nanoTime();
//            for (int i = 0; i < 1000; i++) {
////                lib.check_gate(3);
//                statsig.checkGate(user, gate);
//            }
//            long end = System.nanoTime();
//            times.put(gate, (double) (end - start) / 1_000_000.0);
//        }
//
//        long allEnd = System.nanoTime();
//
//        System.out.println(new Gson().toJson(times));
//        System.out.println("all duration: " + (allEnd - allStart) / 1_000_000.0 + " ms");
//
//
//        options.close();
//        statsig.close();
//
//        System.out.println("StatsigOptions Released: " + options.getRef().isReleased());
//        System.out.println("Statsig Released: " + statsig.getRef().isReleased());
    }

//    private static StatsigUser getUser() {
//        Map<String, String> custom = new HashMap<>();
//        custom.put("flavor", "scrapi-init-cg");
//        custom.put("region", "gke-europe-west1");
//        custom.put("service", "scrapi-init");
//        custom.put("serviceVersion", "240814-61ec7ebf0395");
//        custom.put("tier", "prod");
//
//        // Create the customIDs map
//        Map<String, String> customIDs = new HashMap<>();
//        customIDs.put("podID", "scrapi-init-jpvgc");
//
//        // Create the StatsigUser object
//        return new StatsigUser(
//                null,        // userId
//                customIDs,   // customIds
//                null,        // email
//                null,        // ip
//                null,        // userAgent
//                null,        // country
//                null,        // locale
//                null,        // appVersion
//                custom,      // custom
//                null         // privateAttributes
//        );
//    }
}