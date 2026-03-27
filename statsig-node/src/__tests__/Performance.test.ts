import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

const user = StatsigUser.withUserID('a-user');

const DEFAULT_ITERATIONS = 10_000;
const EXPERIMENT_GET_ITERATIONS = 1_000;
const LAYER_ITERATIONS = 1_000;

type ProfileResult<T> = {
  value: T;
  overall: number;
  p99: number;
  min: number;
  max: number;
};

describe('Performance', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;
  let baseline: ProfileResult<string>;

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcsContent = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/perf_proj_dcs.json',
      ),
      'utf8',
    );

    scrapi.mock('/v2/download_config_specs', dcsContent, {
      status: 200,
      method: 'GET',
    });

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    const options: StatsigOptions = {
      specsUrl: scrapi.getUrlForPath('/v2/download_config_specs'),
      logEventUrl: scrapi.getUrlForPath('/v1/log_event'),
    };

    statsig = new Statsig('secret-key', options);
    await statsig.initialize();

    const data = { foo: 'bar' };
    baseline = profile(() => data.foo);
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('profiles getFeatureGate', () => {
    const results = profile(() => statsig.getFeatureGate(user, 'test_public'));

    printResults('getFeatureGate', results);
    expect(results.overall / baseline.overall).toBeLessThan(1000);
  });

  it('profiles getDynamicConfig', () => {
    const results = profile(() => statsig.getDynamicConfig(user, 'big_dc_base64'));

    printResults('getDynamicConfig', results);
    expect(results.overall / baseline.overall).toBeLessThan(1000);
  });

  it('profiles dynamic config get', () => {
    const config = statsig.getDynamicConfig(user, 'big_dc_base64');

    const results = profile(() => config.get('a', 'err'));
    const value = results.value;

    printResults('getDynamicConfig.get', results);

    expect(value).not.toBe('err');
    expect(results.overall / baseline.overall).toBeLessThan(1000);
  });

  it('gets the expected dynamic config string value', () => {
    const config = statsig.getDynamicConfig(user, 'big_dc_base64');
    const value = config.get('a', 'err');

    expect(value.startsWith('TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGF')).toBe(true);
  });

  it('profiles getExperiment', () => {
    const results = profile(() =>
      statsig.getExperiment(user, 'another_big_expemeriment_nine'),
    );

    printResults('getExperiment', results);
  });

  it('profiles experiment get', () => {
    const experiment = statsig.getExperiment(user, 'another_big_expemeriment_nine');

    const results = profile(
      () => experiment.get('kanto_pokedex_json', 'err'),
      EXPERIMENT_GET_ITERATIONS,
    );
    const value = results.value;

    printResults('experiment.get', results);

    expect(value).not.toBe('err');
    expect(results.overall / baseline.overall).toBeLessThan(1000);
  });

  it('profiles getLayer', () => {
    const results = profile(
      () => statsig.getLayer(user, 'big_layer'),
      LAYER_ITERATIONS,
    );

    printResults('getLayer', results);
  });

  it('profiles layer get', () => {
    const layer = statsig.getLayer(user, 'big_layer');

    const results = profile(() => layer.get('another_object', { err: 1 }));
    const value = results.value;

    printResults('layer.get', results);

    expect(value).not.toEqual({ err: 1 });
    // todo: why is this slow?
    // expect(results.overall / baseline.overall).toBeLessThan(1000);
  });
});

function profile<T>(
  action: () => T,
  iterations: number = DEFAULT_ITERATIONS,
): ProfileResult<T> {
  const overallStart = performance.now();
  const durations: number[] = [];

  let value!: T;

  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    value = action();
    const end = performance.now();
    durations.push(end - start);
  }

  const overallEnd = performance.now();
  const overall = overallEnd - overallStart;

  durations.sort((a, b) => a - b);

  return {
    value,
    overall,
    p99: durations[Math.floor(iterations * 0.99)],
    min: durations[0],
    max: durations[durations.length - 1],
  };
}

function printResults<T>(name: string, result: ProfileResult<T>) {
  const { value: _value, ...summary } = result;
  console.log(`${name} performance:`, JSON.stringify(summary, null, 2));
}
