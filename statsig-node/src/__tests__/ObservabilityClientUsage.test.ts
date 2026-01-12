import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockObservabilityClient } from './MockObservabilityClient';
import { MockScrapi } from './MockScrapi';

describe('ObservabilityClient Usage', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;
  let observabilityClient: MockObservabilityClient;

  let observabilityClientSpies: {
    init: jest.SpyInstance;
    gauge: jest.SpyInstance;
    increment: jest.SpyInstance;
    dist: jest.SpyInstance;
  };

  beforeAll(async () => {
    scrapi = await MockScrapi.create();
    observabilityClient = new MockObservabilityClient();

    let dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/eval_proj_dcs.json',
      ),
      'utf8',
    );

    dcs = dcs.replace(
      '"checksum":"8506699639233708000"',
      '"IGNORED_CHECKSUM_VALUE":""',
    );

    scrapi.mock('/v2/download_config_specs', dcs, {
      status: 200,
      method: 'GET',
    });

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    observabilityClientSpies = {
      init: jest.spyOn(observabilityClient, 'initialize'),
      gauge: jest.spyOn(observabilityClient, 'gauge'),
      increment: jest.spyOn(observabilityClient, 'increment'),
      dist: jest.spyOn(observabilityClient, 'dist'),
    };

    const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
    const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
    const options: StatsigOptions = {
      specsUrl,
      logEventUrl,
      observabilityClient,
      specsSyncIntervalMs: 1,
    };

    statsig = new Statsig('secret-123', options);
    await statsig.initialize();

    statsig.checkGate(StatsigUser.withUserID('test-user'), 'test-gate');
    statsig.logEvent(StatsigUser.withUserID('b-user'), 'my_event');

    await statsig.flushEvents();

    await waitFor(
      () => observabilityClientSpies.dist.mock.calls.length > 1, // init + config prop
      5000,
    );

    scrapi.mock('/v2/download_config_specs', '{"has_updates": false}', {
      status: 200,
      method: 'GET',
    });

    await scrapi.waitForNext((req) =>
      req.path.includes('/v2/download_config_specs'),
    );

    await waitFor(
      () => observabilityClientSpies.increment.mock.calls.length > 1, // no updates
      5000,
    );
  }, 10000);

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('logs a dist metric for sdk initialization time', () => {
    expect(observabilityClientSpies.dist).toHaveBeenCalledWith(
      'statsig.sdk.initialization',
      expect.any(Number),
      {
        success: 'true',
        store_populated: 'true',
        source: 'Network',
        spec_source_api: scrapi.getServerApi(),
      },
    );
  });

  it('logs a dist metric for config propagation time', () => {
    expect(observabilityClientSpies.dist).toHaveBeenCalledWith(
      'statsig.sdk.config_propagation_diff',
      expect.any(Number),
      {
        source: 'Network',
        spec_source_api: scrapi.getServerApi(),
        response_format: 'JSON',
      },
    );
  });

  it('logs an increment log events', () => {
    expect(observabilityClientSpies.increment).toHaveBeenCalledWith(
      'statsig.sdk.events_successfully_sent_count',
      3,
      null,
    );
  });

  it('logs config no update', () => {
    expect(observabilityClientSpies.increment).toHaveBeenCalledWith(
      'statsig.sdk.config_no_update',
      1,
      { source: 'Network', spec_source_api: scrapi.getServerApi() },
    );
  });
});

async function waitFor(fn: () => boolean, timeout: number) {
  const startTime = Date.now();
  while (!fn() && Date.now() - startTime < timeout) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}
