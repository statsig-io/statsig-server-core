package example.statsig;

import com.statsig.OutputLogger;
import com.statsig.Statsig;
import com.statsig.StatsigOptions;
import com.statsig.StatsigUser;

import java.util.concurrent.ExecutionException;


public class Main {
    public static void main(String[] args) throws ExecutionException, InterruptedException {
        StatsigOptions options = new StatsigOptions.Builder().setOutputLoggerLevel(OutputLogger.LogLevel.DEBUG).build();
        String sdkKey = System.getenv("test_api_key");
        Statsig statsig = new Statsig(sdkKey, options);

        statsig.initialize().get();

        StatsigUser user = new StatsigUser.Builder().setUserID("a_user").build();

        boolean check = statsig.checkGate(user, "test_public");

        System.out.println("test_public: " + check);

        statsig.shutdown().get();
    }
}