import {
  Statsig,
  StatsigOptions,
  StatsigUser,
} from '@statsig/statsig-node-core';
import { version as sdkVersion } from '@statsig/statsig-node-core/package.json';
import fs from 'node:fs';

const sdkType = 'statsig-server-core-node';

const SCRAPI_URL = 'http://scrapi:8000';
const ITER_LITE = 1000;
const ITER_HEAVY = 10_000;

type BenchmarkResult = {
  p99: number;
  max: number;
  min: number;
  median: number;
  avg: number;
  benchmarkName: string;
  specName: string;
  sdkType: string;
  sdkVersion: string;
};

// ------------------------------------------------------------------------ [ Setup ]

const specNamePath = '/shared-volume/spec_names.json';
for (let i = 0; i < 10; i++) {
  if (fs.existsSync(specNamePath)) {
    break;
  }

  await new Promise((resolve) => setTimeout(resolve, 1000));
}

const specNames = JSON.parse(fs.readFileSync(specNamePath, 'utf8'));

const options: StatsigOptions = {
  specsUrl: `${SCRAPI_URL}/v2/download_config_specs`,
  logEventUrl: `${SCRAPI_URL}/v1/log_event`,
};

const statsig = new Statsig('secret-NODE_CORE', options);
await statsig.initialize();

const results: BenchmarkResult[] = [];

const benchmark = async <T,>(
  benchName: string,
  specName: string,
  iterations: number,
  func: () => Promise<T> | T,
  cleanup?: (result: T) => void,
) => {
  const durations: number[] = [];

  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    const result = await func();
    const end = performance.now();
    durations.push(end - start);

    if (cleanup) {
      cleanup(result);
    }
  }

  // Calculate p99
  durations.sort((a, b) => a - b);
  const p99Index = Math.floor(iterations * 0.99);

  const result = {
    benchmarkName: benchName,
    p99: durations[p99Index],
    max: durations[durations.length - 1],
    min: durations[0],
    median: durations[Math.floor(durations.length / 2)],
    avg: durations.reduce((a, b) => a + b, 0) / durations.length,
    specName,
    sdkType,
    sdkVersion,
  };

  results.push(result);

  console.log(
    `${benchName.padEnd(30)}`,
    `p99(${result.p99.toFixed(4)}ms)`.padEnd(15),
    `max(${result.max.toFixed(4)}ms)`.padEnd(15),
    specName,
  );

  await new Promise((resolve) => setTimeout(resolve, 1));
};

const createUser = () => {
  return new StatsigUser({
    userID: Math.random().toString(36).substring(2, 15),
    email: 'user@example.com',
    ip: '127.0.0.1',
    locale: 'en-US',
    appVersion: '1.0.0',
    country: 'US',
    userAgent:
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
    custom: {
      isAdmin: false,
    },
    privateAttributes: {
      isPaid: 'nah',
    },
  });
};

// ------------------------------------------------------------------------ [ Benchmark ]

console.log(`Statsig Node Core (v${sdkVersion})`);
console.log('--------------------------------');

const globalUser = StatsigUser.withUserID('global_user');

await benchmark(
  'initialize',
  'n/a',
  ITER_LITE,
  async () => {
    const inst = new Statsig('secret-NODE_CORE', options);
    await inst.initialize();
    return inst;
  },
  (inst: Statsig) => {
    inst.shutdown().catch((err) => {
      console.error(err);
    });
  },
);

for (const gate of specNames.feature_gates) {
  await benchmark('check_gate', gate, ITER_HEAVY, () => {
    const user = createUser();
    statsig.checkGate(user, gate);
  });

  await benchmark('check_gate_global_user', gate, ITER_HEAVY, () => {
    statsig.checkGate(globalUser, gate);
  });

  await benchmark('get_feature_gate', gate, ITER_HEAVY, () => {
    const user = createUser();
    statsig.getFeatureGate(user, gate);
  });

  await benchmark('get_feature_gate_global_user', gate, ITER_HEAVY, () => {
    statsig.getFeatureGate(globalUser, gate);
  });
}

for (const config of specNames.dynamic_configs) {
  await benchmark('get_dynamic_config', config, ITER_HEAVY, () => {
    const user = createUser();
    statsig.getDynamicConfig(user, config);
  });

  await benchmark('get_dynamic_config_global_user', config, ITER_HEAVY, () => {
    statsig.getDynamicConfig(globalUser, config);
  });

   // Get parameter names from the actual config
   const sampleConfig = statsig.getDynamicConfig(globalUser, config);
   const paramNames = Object.keys(sampleConfig.value);
   
   // If no parameters, skip get() benchmarks
   if (paramNames.length === 0) {
     continue;
   }
 
   // Benchmark getValue for each parameter
   for (const paramName of paramNames) {
     await benchmark(
       `get_dynamic_config_getValue_global_user`,
       `${config}.${paramName}`,
       ITER_HEAVY,
       () => {
         const cfg = statsig.getDynamicConfig(globalUser, config).getValue(paramName);
       },
     );
 
     // Test get() method (typed version)
     await benchmark(
       `get_dynamic_config_get_global_user`,
       `${config}.${paramName}`,
       ITER_HEAVY,
       () => {
         const cfg = statsig.getDynamicConfig(globalUser, config).get(paramName, undefined);
       },
     );
   }
}

for (const experiment of specNames.experiments) {
  await benchmark('get_experiment', experiment, ITER_HEAVY, () => {
    const user = createUser();
    statsig.getExperiment(user, experiment);
  });

  await benchmark('get_experiment_global_user', experiment, ITER_HEAVY, () => {
    statsig.getExperiment(globalUser, experiment);
  });
}

for (const layer of specNames.layers) {
  await benchmark('get_layer', layer, ITER_HEAVY, () => {
    const user = createUser();
    statsig.getLayer(user, layer);
  });

  await benchmark('get_layer_global_user', layer, ITER_HEAVY, () => {
    statsig.getLayer(globalUser, layer);
  });
}

await benchmark('get_client_initialize_response', 'n/a', ITER_LITE, () => {
  statsig.getClientInitializeResponse(createUser());
});

await benchmark(
  'get_client_initialize_response_global_user',
  'n/a',
  ITER_LITE,
  () => {
    statsig.getClientInitializeResponse(globalUser);
  },
);

// ------------------------------------------------------------------------ [ Teardown ]

await statsig.shutdown();

fs.writeFileSync(
  `/shared-volume/${sdkType}-${sdkVersion}-results.json`,
  JSON.stringify(
    {
      sdkType,
      sdkVersion,
      results,
    },
    null,
    2,
  ),
);
