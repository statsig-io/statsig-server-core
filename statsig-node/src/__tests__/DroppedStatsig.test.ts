/**
 * @jest-environment node
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import { setFlagsFromString } from 'node:v8';
import { runInNewContext } from 'node:vm';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

setFlagsFromString('--expose_gc');
const gc = runInNewContext('gc');

async function runGarbageCollection() {
  gc();

  // Delay to ensure GC has completed
  await new Promise((resolve) => setTimeout(resolve, 100));
}

describe('Statsig', () => {
  let options: StatsigOptions;
  let scrapi: MockScrapi;

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/eval_proj_dcs.json',
      ),
      'utf8',
    );

    scrapi.mock('/v2/download_config_specs', dcs, {
      status: 200,
      method: 'GET',
    });

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
    const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
    options = {
      specsUrl,
      logEventUrl,
      specsSyncIntervalMs: 1,
    };
  });

  afterAll(async () => {
    scrapi.close();
  });

  it('should automatically shutdown the statsig instance when it is garbage collected', async () => {
    const globalStatsig = new Statsig('secret-global', options);
    await globalStatsig.initialize();

    async function createAndDrop(sdkKey: string) {
      const statsig = new Statsig(sdkKey, options);
      await statsig.initialize();

      const user = new StatsigUser({
        userID: 'test-user',
      });
      statsig.logEvent(user, 'test-event', 'test-value');

      // intentionally not calling shutdown on these instances
    }

    await Promise.all([
      createAndDrop('secret-1'),
      createAndDrop('secret-2'),
      createAndDrop('secret-3'),
    ]);

    await runGarbageCollection();

    scrapi.requests = [];

    let i = 0;
    while (scrapi.requests.length == 0 && i < 10) {
      await new Promise((resolve) => setTimeout(resolve, 100));
      i++;
    }

    await globalStatsig.shutdown();

    // any dcs call not from the global instance
    const badReqs = scrapi.requests.filter(
      (req) =>
        req.path.includes('download_config_specs') &&
        !req.path.includes('secret-global'),
    );

    expect(badReqs).toHaveLength(0);

    const goodReqs = scrapi.requests.filter(
      (req) =>
        req.path.includes('download_config_specs') &&
        req.path.includes('secret-global'),
    );

    expect(goodReqs.length).toBeGreaterThan(0);
  });
});
