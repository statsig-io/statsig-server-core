import 'jest-extended';
import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

const ITER = 100_000;
const CONFIG_NAME = 'big_dc_base64';
const KNOWN_STRING_PARAM = 'a';

// Please do NOT increase this threshold without understanding why
// The bug that was fixed was reporting as ~70x slower than the baseline
const EXPECT_LOW = 0.3;
const EXPECT_HIGH = 1.6;

type ProfileResult = {
  overall: number;
  median: number;
  p99: number;
  min: number;
  max: number;
};

describe('Get Dynamic Config Performance', () => {
  const user = StatsigUser.withUserID('a-user');

  let dynamicConfigs: Record<string, any>;
  let statsig: Statsig;
  let scrapi: MockScrapi;
  let baseline: ProfileResult;

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcsData = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/perf_proj_dcs.json',
      ),
      'utf8',
    );

    scrapi.mock('/v2/download_config_specs', dcsData, {
      status: 200,
      method: 'GET',
    });
    const dcs = JSON.parse(dcsData);
    dynamicConfigs = dcs.dynamic_configs;

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
    const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
    const options: StatsigOptions = {
      specsUrl,
      logEventUrl,
      environment: 'development',
    };

    if (dynamicConfigs[CONFIG_NAME].defaultValue['a'] == null) {
      throw new Error('a is null');
    }

    statsig = new Statsig('secret-123', options);
    await statsig.initialize();

    baseline = profile(() => {
      // direct 'Record' access performance
      const dc = dynamicConfigs[CONFIG_NAME];
      const _a = dc.defaultValue[KNOWN_STRING_PARAM];
    });
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('should read parameter value within reasonable time', () => {
    const config = statsig.getDynamicConfig(user, CONFIG_NAME);
    const result = profile(() => config.getValue(KNOWN_STRING_PARAM, {}));

    const overallDelta = result.overall / baseline.overall;
    const p99Delta = result.p99 / baseline.p99;

    expect(overallDelta).toBeWithin(EXPECT_LOW, EXPECT_HIGH);
    expect(p99Delta).toBeWithin(EXPECT_LOW, EXPECT_HIGH);
  });

  it('should access the value field directly within reasonable time', () => {
    const config = statsig.getDynamicConfig(user, CONFIG_NAME);
    const result = profile(() => config.value[KNOWN_STRING_PARAM]);

    const overallDelta = result.overall / baseline.overall;
    const p99Delta = result.p99 / baseline.p99;

    expect(overallDelta).toBeWithin(EXPECT_LOW, EXPECT_HIGH);
    expect(p99Delta).toBeWithin(EXPECT_LOW, EXPECT_HIGH);
  });

  it('should be getting the correct object value', () => {
    const config = statsig.getDynamicConfig(user, CONFIG_NAME);

    const param = config.get(KNOWN_STRING_PARAM, 'err');
    expect(param.startsWith('TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGF')).toBeTruthy();
    expect(Object.keys(config.value)).toEqual(['a']);
  });
});

function profile(action: () => void): ProfileResult {
  const overallStart = performance.now();
  const durations: number[] = [];
  for (let i = 0; i < ITER; i++) {
    const start = performance.now();
    action();
    const end = performance.now();
    durations.push(end - start);
  }

  const overallEnd = performance.now();
  const overall = overallEnd - overallStart;

  const sortedDurations = durations.sort((a, b) => a - b);
  const median = sortedDurations[Math.floor(sortedDurations.length / 2)];
  const p99 = sortedDurations[Math.floor(sortedDurations.length * 0.99)];
  const min = sortedDurations[0];
  const max = sortedDurations[sortedDurations.length - 1];

  return { overall, median, p99, min, max };
}
