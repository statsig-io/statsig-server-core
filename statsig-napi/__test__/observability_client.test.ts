import test from "ava";
import { IObservabilityClient, LogLevel, StatsigOptions, StatsigUser } from "../src";
import { Statsig } from "../dist/lib";

class FakeObClient implements IObservabilityClient  {
  method_called: String[] = [];

  init(): void {
    this.method_called.push("dist");
  }
  increment(metricName: string, value: number, tags: Record<string, any>): void {
    this.method_called.push("increment");
  }
  gauge(metricName: string, value: number, tags: Record<string, any>): void {
    this.method_called.push("gauge");
  }
  dist(metricName: string, value: number, tags: Record<string, any>): void {
    this.method_called.push("dist");
  }
  should_enable_high_cardinality_for_this_tag?(tag: string): void {
    this.method_called.push("should_enable_high_cardinality_for_this_tag");
  }

}

test('Usage Example',async (t) => {
  const obClient = new FakeObClient()
  const user = new StatsigUser({userID: "test-user", customIDs: {}});
  const statsigOptions = new StatsigOptions({
    outputLoggerLevel: LogLevel.Debug,
    environment: 'staging',
    observabilityClient: obClient
  })
  const statsig = new Statsig("secret-key", statsigOptions);
  statsig.initialize();
  t.is(obClient.method_called.includes("dist"), true);
})