import fs from 'node:fs';
import Statsig, { StatsigUser } from 'statsig-node';
import { version } from 'statsig-node/package.json';

const key = process.env.PERF_SDK_KEY!;
const sdkVersion = version;
const sdkType = 'statsig-node';

const metadataFile = process.env.BENCH_METADATA_FILE!;
fs.writeFileSync(
  metadataFile,
  JSON.stringify({
    sdk_type: sdkType,
    sdk_version: sdkVersion,
  }),
);

await Statsig.initialize(key);

const CORE_ITER = 100_000;
const GCIR_ITER = 1000;

const globalUser: StatsigUser = {
  userID: 'global_user',
};

const results: Record<string, number> = {};

const makeRandomUser = () => {
  return {
    userID: Math.random().toString(36).substring(2, 15),
  };
};

const benchmark = (iterations: number, func: () => void | Promise<void>) => {
  const durations: number[] = [];

  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    func();
    const end = performance.now();
    durations.push(end - start);
  }

  // Calculate p99
  durations.sort((a, b) => a - b);
  const p99Index = Math.floor(iterations * 0.99);
  return durations[p99Index];
};

const logBenchmark = (name: string, p99: number) => {
  console.log(name.padEnd(50), p99.toFixed(4) + 'ms');

  const ci = process.env.CI;
  if (ci !== '1' && ci !== 'true') {
    return;
  }

  Statsig.logEvent(globalUser, 'sdk_benchmark', p99, {
    benchmarkName: name,
    sdkType,
    sdkVersion,
  });
};

const runCheckGate = () => {
  const p99 = benchmark(CORE_ITER, () => {
    Statsig.checkGate(makeRandomUser(), 'test_advanced');
  });
  results['check_gate'] = p99;
};

const runCheckGateGlobalUser = () => {
  const p99 = benchmark(CORE_ITER, () => {
    Statsig.checkGate(globalUser, 'test_advanced');
  });
  results['check_gate_global_user'] = p99;
};

const runGetFeatureGate = () => {
  const p99 = benchmark(CORE_ITER, () => {
    Statsig.getFeatureGate(makeRandomUser(), 'test_advanced');
  });
  results['get_feature_gate'] = p99;
};

const runGetFeatureGateGlobalUser = () => {
  const p99 = benchmark(CORE_ITER, () => {
    Statsig.getFeatureGate(globalUser, 'test_advanced');
  });
  results['get_feature_gate_global_user'] = p99;
};

const runGetExperiment = () => {
  const p99 = benchmark(CORE_ITER, () => {
    Statsig.getExperiment(makeRandomUser(), 'an_experiment');
  });
  results['get_experiment'] = p99;
};

const runGetExperimentGlobalUser = () => {
  const p99 = benchmark(CORE_ITER, () => {
    Statsig.getExperiment(globalUser, 'an_experiment');
  });
  results['get_experiment_global_user'] = p99;
};

const runGetClientInitializeResponse = () => {
  const p99 = benchmark(GCIR_ITER, () => {
    const res = Statsig.getClientInitializeResponse(makeRandomUser());
    JSON.stringify(res);
  });
  results['get_client_initialize_response'] = p99;
};

const runGetClientInitializeResponseGlobalUser = () => {
  const p99 = benchmark(GCIR_ITER, () => {
    const res = Statsig.getClientInitializeResponse(globalUser);
    JSON.stringify(res);
  });
  results['get_client_initialize_response_global_user'] = p99;
};

console.log(`Statsig Node Legacy (v${sdkVersion})`);
console.log('--------------------------------');

runCheckGate();
runCheckGateGlobalUser();
runGetFeatureGate();
runGetFeatureGateGlobalUser();
runGetExperiment();
runGetExperimentGlobalUser();
runGetClientInitializeResponse();
runGetClientInitializeResponseGlobalUser();

for (const [name, p99] of Object.entries(results)) {
  logBenchmark(name, p99);
}

Statsig.shutdown();
