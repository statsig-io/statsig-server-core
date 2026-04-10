import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Layer Param Exposure Sampling', () => {
  let scrapi: MockScrapi;
  let statsig: Statsig;

  const getLoggedLayerExposures = (): Record<string, any>[] => {
    return scrapi.requests
      .flatMap((request) => request.body?.events ?? [])
      .filter(
        (event: Record<string, any>) =>
          event.eventName === 'statsig::layer_exposure',
      );
  };

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/dcs_with_analytical_exposure_sampling.json',
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

    const options: StatsigOptions = {
      specsUrl: scrapi.getUrlForPath('/v2/download_config_specs'),
      logEventUrl: scrapi.getUrlForPath('/v1/log_event'),
      outputLogLevel: 'none',
    };

    statsig = new Statsig('secret-123', options);
    await statsig.initialize();
    scrapi.clearRequests();
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  beforeEach(() => {
    scrapi.clearRequests();
  });

  it('samples layer param exposures when raw layer exposure data crosses the Node bridge', async () => {
    for (let i = 0; i < 80; i++) {
      const user = StatsigUser.withUserID(`sampled-layer-user-${i}`);
      const layer = statsig.getLayer(user, 'json_sampled_layer');

      expect(layer.get('param', 'fallback')).toBe('layer_value');
    }

    await statsig.flushEvents();

    const events = getLoggedLayerExposures();
    expect(events.length).toBeLessThan(20);
    expect(
      events.every((event) => event.metadata?.config === 'json_sampled_layer'),
    ).toBe(true);
  });

  it('logs every sampled layer param exposure after evaluating an analytical gate through Node', async () => {
    for (let i = 0; i < 40; i++) {
      const user = StatsigUser.withUserID(`analytical-layer-user-${i}`);
      const layer = statsig.getLayer(user, 'parent_layer');

      expect(layer.get('param', 'fallback')).toBe('layer_value');
    }

    await statsig.flushEvents();

    const events = getLoggedLayerExposures();
    expect(events).toHaveLength(40);
    expect(events.every((event) => event.metadata?.config === 'parent_layer'))
      .toBe(true);
  });
});
