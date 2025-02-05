import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Statsig', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;

  const getLastLoggedEvent = async (): Promise<Record<string, any> | null> => {
    await statsig.flushEvents();
    if (scrapi.requests.length === 0) {
      return null;
    }

    const request = scrapi.requests[0];

    if (!request.body.events) {
      return null;
    }

    const events = request.body.events;
    return (
      events.filter((e: any) => e.eventName !== 'statsig::diagnostics')[0] ??
      null
    );
  };

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-lib/tests/data/eval_proj_dcs.json',
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
    const options: StatsigOptions = {
      specsUrl,
      logEventUrl,
    };

    statsig = new Statsig('secret-123', options);
    await statsig.initialize();
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('makes a request to download the config specs', async () => {
    const request = scrapi.requests[0];
    expect(request.path).toEqual('/v2/download_config_specs/secret-123.json');
  });

  it('should get the client init response', async () => {
    const user = StatsigUser.withUserID('a-user');
    const response = JSON.parse(statsig.getClientInitializeResponse(user));

    expect(Object.keys(response.feature_gates)).toHaveLength(65);
    expect(Object.keys(response.dynamic_configs)).toHaveLength(62);
    expect(Object.keys(response.layer_configs)).toHaveLength(12);
  });

  describe('checks, events and exposures', () => {
    beforeEach(async () => {
      await statsig.flushEvents();
      scrapi.requests.length = 0;
    });

    it('should log custom events', async () => {
      const user = StatsigUser.withUserID('a-user');
      statsig.logEvent(user, 'my_custom_event', 'my_value');

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('my_custom_event');
      expect(event?.value).toEqual('my_value');
    });

    it('should check gates and log exposures', async () => {
      const user = StatsigUser.withUserID('a-user');
      const gate = statsig.checkGate(user, 'test_public');

      expect(gate).toBe(true);

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::gate_exposure');
      expect(event?.metadata?.gate).toEqual('test_public');
    });

    it('should get feature gates and log exposures', async () => {
      const user = StatsigUser.withUserID('b-user');
      const gate = statsig.getFeatureGate(user, 'test_public');

      expect(gate.value).toBe(true);

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::gate_exposure');
      expect(event?.metadata?.gate).toEqual('test_public');
    });

    it('should get dynamic configs and log exposures', async () => {
      const user = StatsigUser.withUserID('a-user');
      const config = statsig.getDynamicConfig(user, 'operating_system_config');

      expect(config.value).toEqual({
        num: 13,
        bool: true,
        str: 'hello',
        arr: ['hi', 'there'],
        obj: { a: 'bc' },
      });

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::config_exposure');
      expect(event?.metadata?.config).toEqual('operating_system_config');
    });

    it('should get experiments and log exposures', async () => {
      const user = StatsigUser.withUserID('a-user');
      const experiment = statsig.getExperiment(user, 'exp_with_obj_and_array');

      expect(experiment.value).toEqual({
        arr_param: [true, false, true],
        obj_param: {
          group: 'test',
        },
      });

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::config_exposure');
      expect(event?.metadata?.config).toEqual('exp_with_obj_and_array');
    });

    it('should get layers and log exposures', async () => {
      const user = StatsigUser.withUserID('a-user');
      const layer = statsig.getLayer(user, 'layer_with_many_params');

      let event = await getLastLoggedEvent();
      expect(event).toBeNull();

      const value = layer.get('another_string', 'err');
      expect(value).toEqual('layer_default');

      event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::layer_exposure');
      expect(event?.metadata?.config).toEqual('layer_with_many_params');
    });
  });
});
