import com.statsig.*;

import java.util.Map;
import java.util.Objects;

public class StatsigWrapper {
    public static final String SCRAPI_URL = "http://scrapi:8000";

    private static boolean _isCore;
    private static Statsig _statsig;
    private static StatsigUser _coreUser;
    private static com.statsig.sdk.StatsigUser _legacyUser;

    public static void initialize() throws Exception {
        String variant = System.getenv("SDK_VARIANT");

        if (Objects.equals(variant, "core")) {
            _isCore = true;

            StatsigOptions options = new StatsigOptions.Builder()
                    .setSpecsUrl(SCRAPI_URL + "/v2/download_config_specs")
                    .setLogEventUrl(SCRAPI_URL + "/v1/log_event")
                    .build();

            _statsig = new Statsig("secret-JAVA_CORE", options);
            _statsig.initialize().get();
        } else if (Objects.equals(variant, "legacy")) {
            _isCore = false;

            com.statsig.sdk.StatsigOptions options = new com.statsig.sdk.StatsigOptions();
            options.setApi(SCRAPI_URL + "/v1");
            options.setLogLevel(com.statsig.sdk.LogLevel.DEBUG);

            com.statsig.sdk.Statsig.initializeAsync("secret-JAVA_LEGACY", options);
        } else {
            throw new IllegalArgumentException("Invalid SDK variant: " + variant);
        }
    }

    public static void setUser(Map<String, String> userData) throws Exception {
        if (_isCore) {
            _coreUser = new StatsigUser.Builder().setUserID("global_user").build();
        } else {
            _legacyUser = new com.statsig.sdk.StatsigUser("global_user");
        }
    }

    public static void checkGate(String gateName) {
        if (_isCore) {
            _statsig.checkGate(_coreUser, gateName);
        }  else {
            com.statsig.sdk.Statsig.checkGateSync(_legacyUser, gateName);
        }
    }

    public static void logEvent(String eventName) {
        if (_isCore) {
            _statsig.logEvent(_coreUser, eventName);
        }  else {
            com.statsig.sdk.Statsig.logEvent(_legacyUser, eventName);
        }
    }
}