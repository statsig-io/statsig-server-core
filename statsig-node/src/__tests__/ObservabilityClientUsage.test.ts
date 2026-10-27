import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockObservabilityClient } from './MockObservabilityClient';
import { MockScrapi } from './MockScrapi';

const TITLE = 'ObservabilityClient Usage (split batches: %s)';

describe.each([false, true])(TITLE, (splitBatches) => {
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

    const flushUntilDiagnostic = () =>
      waitFor(async () => {
        await statsig.flushEvents();
        return scrapi
          .getLogEventRequests()
          .some((request) =>
            request.body.events.some(
              (event: { eventName: string }) =>
                event.eventName === 'statsig::diagnostics',
            ),
          );
      }, 5000);

    if (splitBatches) {
      await flushUntilDiagnostic();
    }

    statsig.checkGate(StatsigUser.withUserID('test-user'), 'test-gate');
    statsig.logEvent(StatsigUser.withUserID('b-user'), 'my_event');

    // Initialization diagnostics reach the event queue through an async observer;
    // initialize() need not put them in the same batch as the explicit events.
    await flushUntilDiagnostic();

    await waitFor(
      () =>
        observabilityClientSpies.increment.mock.calls
          .filter(
            ([name]) => name === 'statsig.sdk.events_successfully_sent_count',
          )
          .reduce((sum, [, count]) => sum + count, 0) === 3,
      5000,
    );

    await waitFor(
      () =>
        observabilityClientSpies.dist.mock.calls.some(
          ([metricName]) => metricName === 'statsig.sdk.initialization',
        ),
      5000,
    );

    await waitFor(
      () =>
        observabilityClientSpies.dist.mock.calls.some(
          ([metricName]) =>
            metricName === 'statsig.sdk.config_propagation_diff',
        ),
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
      () =>
        observabilityClientSpies.increment.mock.calls.some(
          ([name]) => name === 'statsig.sdk.config_no_update',
        ),
      5000,
    );
  }, 10000);

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('logs a dist metric for sdk initialization time', () => {
    const initCall = observabilityClientSpies.dist.mock.calls.find(
      ([metricName]) => metricName === 'statsig.sdk.initialization',
    );

    expect(initCall).toBeDefined();
    expect(initCall?.[1]).toEqual(expect.any(Number));
    expect(initCall?.[2]).toEqual(
      expect.objectContaining({
        init_success: 'true',
        store_populated: 'true',
        source: 'Network',
        sdk_key: 'secret-123',
        sdk_type: 'statsig-server-core-node',
        sdk_version: expect.any(String),
        init_source_api: scrapi.getServerApi() + '/v2',
      }),
    );
  });

  it('logs a dist metric for config propagation time', () => {
    expect(observabilityClientSpies.dist).toHaveBeenCalledWith(
      'statsig.sdk.config_propagation_diff',
      expect.any(Number),
      {
        source: 'Network',
        source_api: scrapi.getServerApi() + '/v2',
        response_format: 'JSON',
      },
    );
  });

  it('logs an increment log events', () => {
    const events = scrapi
      .getLogEventRequests()
      .flatMap((request) => request.body.events);
    expect(events.map((event) => event.eventName).sort()).toEqual(
      ['statsig::gate_exposure', 'my_event', 'statsig::diagnostics'].sort(),
    );
    if (splitBatches) {
      expect(scrapi.getLogEventRequests().length).toBeGreaterThanOrEqual(2);
    }

    const sentCounts = observabilityClientSpies.increment.mock.calls.filter(
      ([name]) => name === 'statsig.sdk.events_successfully_sent_count',
    );
    expect(sentCounts.reduce((sum, [, count]) => sum + count, 0)).toBe(
      events.length,
    );
    for (const [, , tags] of sentCounts) {
      expect(tags).toBeNull();
    }
  });

  it('logs config no update', () => {
    expect(observabilityClientSpies.increment).toHaveBeenCalledWith(
      'statsig.sdk.config_no_update',
      1,
      { source: 'Network', source_api: scrapi.getServerApi() + '/v2' },
    );
  });
});

async function waitFor(fn: () => boolean | Promise<boolean>, timeout: number) {
  const startTime = Date.now();
  while (Date.now() - startTime < timeout) {
    if (await fn()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(`Timed out waiting for observability after ${timeout}ms`);
}
