
package example.statsig;

import com.amazonaws.services.lambda.runtime.Context;
import com.amazonaws.services.lambda.runtime.RequestHandler;
import com.statsig.Statsig;
import com.statsig.StatsigOptions;
import com.statsig.StatsigUser;

import java.util.Map;
import java.util.concurrent.ExecutionException;

public class StatsigLambdaHandler implements RequestHandler<Map<String, Object>, String> {

    @Override
    public String handleRequest(Map<String, Object> stringObjectMap, Context context) {
        Statsig statsig = new Statsig("secret-fake", new StatsigOptions.Builder().build());

        try {
            statsig.initialize().get();
        } catch (InterruptedException e) {
            throw new RuntimeException(e);
        } catch (ExecutionException e) {
            throw new RuntimeException(e);
        }

        StatsigUser user = new StatsigUser.Builder()
               .setUserID("user123")
               .setEmail("test@example.com")
               .build();

        try {
            statsig.shutdown().get();
        } catch (InterruptedException e) {
            throw new RuntimeException(e);
        } catch (ExecutionException e) {
            throw new RuntimeException(e);
        }
        return "Hello World";
    }
}