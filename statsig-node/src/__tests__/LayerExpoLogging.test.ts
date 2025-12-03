import * as fs from 'node:fs';
import * as path from 'node:path';

import {
  Layer,
  Statsig,
  StatsigOptions,
  StatsigUser,
} from '../../build/index.js';
import { MockScrapi, anyNumericString } from './MockScrapi';

describe('Layer Exposure Logging', () => {
  let scrapi: MockScrapi;
  let events: Record<string, any> | null = null;
  let layer: Layer;
  let value: string;

  const getLoggedEvents = async (): Promise<Record<string, any>[]> => {
    if (scrapi.requests.length === 0) {
      return [];
    }

    const events = scrapi.requests.flatMap((request) => {
      if (!request?.body?.events) {
        return [];
      }

      return request.body.events;
    });

    return events.filter(
      (e: any) => e?.eventName && e.eventName !== 'statsig::diagnostics',
    );
  };

  const runTest = async (
    user: StatsigUser,
    scrapi: MockScrapi,
    paramName?: string,
  ) => {
    const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
    const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
    const options: StatsigOptions = {
      specsUrl,
      logEventUrl,
      outputLogLevel: 'debug',
    };

    const statsig = new Statsig('secret-123', options);
    await statsig.initialize();

    const layer = statsig.getLayer(user, 'test_layer_in_holdout');
    const value = layer.get(paramName ?? 'exp_val', 'err');

    scrapi.clearRequests();
    await statsig.flushEvents();

    const events = await getLoggedEvents();

    await statsig.shutdown();

    return { value, events, layer };
  };

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
  });

  afterAll(async () => {
    scrapi.close();
  });

  describe('a user in the test group', () => {
    beforeAll(async () => {
      const userInTest = StatsigUser.withUserID('user-in-test');

      const {
        layer: layerResult,
        value: valueResult,
        events: eventsResult,
      } = await runTest(userInTest, scrapi);

      layer = layerResult;
      value = valueResult;
      events = eventsResult;
    });

    it('should be in the test group', async () => {
      expect(layer.getGroupName()).toBe('Test');
      expect(value).toBe('running_test');
      expect(layer.getAllocatedExperimentName()).toBe(
        'running_exp_in_layer_with_holdout',
      );
      expect(layer.getRuleId()).toBe('FC34CQnbBwlkcpMxdi8MT');
    });

    it('should log a layer exposure event', async () => {
      expect(events).toHaveLength(1);

      const event = events?.[0] as Record<string, any>;

      expect(event.eventName).toBe('statsig::layer_exposure');
      expect(event.metadata).toMatchObject({
        config: 'test_layer_in_holdout',
        ruleID: 'FC34CQnbBwlkcpMxdi8MT',
        allocatedExperiment: 'running_exp_in_layer_with_holdout',
        isExplicitParameter: 'true',
        lcut: anyNumericString(),
        receivedAt: anyNumericString(),
        configVersion: '5',
      });
    });

    it('should contain the expected secondary exposures', async () => {
      const event = events?.[0] as Record<string, any>;
      expect(event?.secondaryExposures).toHaveLength(4);
      expect(event?.secondaryExposures).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            gate: 'global_holdout',
            gateValue: 'false',
            ruleID: '3QoA4ncNdVGBaMt3N1KYjz:0.50:1',
          }),
          expect.objectContaining({
            gate: 'layer_holdout',
            gateValue: 'false',
            ruleID: '2bAVp6R3C85vCYrR6be36n:10.00:5',
          }),
        ]),
      );
    });
  });

  describe('a user in the control group', () => {
    beforeAll(async () => {
      const userInControl = StatsigUser.withUserID('user-in-control-1');
      const {
        layer: layerResult,
        value: valueResult,
        events: eventsResult,
      } = await runTest(userInControl, scrapi);
      layer = layerResult;
      value = valueResult;
      events = eventsResult;
    });

    it('should be in the control group', async () => {
      expect(layer.getGroupName()).toBe('Control');
      expect(value).toBe('control');
    });

    it('should log a layer exposure event', async () => {
      expect(events).toHaveLength(1);

      const event = events?.[0] as Record<string, any>;
      expect(event.eventName).toBe('statsig::layer_exposure');

      expect(event.metadata).toMatchObject({
        config: 'test_layer_in_holdout',
        ruleID: '6h3c5q1Q6pkA5BUg7tuIae',
        allocatedExperiment: 'targeted_exp_in_layer_with_holdout',
        isExplicitParameter: 'true',
      });
    });

    it('should contain the expected secondary exposures', async () => {
      const event = events?.[0] as Record<string, any>;
      expect(event?.secondaryExposures).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            gate: 'global_holdout',
            gateValue: 'false',
            ruleID: '3QoA4ncNdVGBaMt3N1KYjz:0.50:1',
          }),
          expect.objectContaining({
            gate: 'layer_holdout',
            gateValue: 'false',
            ruleID: '2bAVp6R3C85vCYrR6be36n:10.00:5',
          }),
        ]),
      );
    });
  });

  describe('a user in the test group accessing a layer default parameter', () => {
    beforeAll(async () => {
      const userInTest = StatsigUser.withUserID('user-in-test');

      const {
        layer: layerResult,
        value: valueResult,
        events: eventsResult,
      } = await runTest(userInTest, scrapi, 'layer_val');

      layer = layerResult;
      value = valueResult;
      events = eventsResult;
    });

    it('should be in the test group', async () => {
      expect(layer.getGroupName()).toBe('Test');
      expect(value).toBe('layer_default');
    });

    it('should log a layer exposure event', async () => {
      expect(events).toHaveLength(1);

      const event = events?.[0] as Record<string, any>;

      expect(event.eventName).toBe('statsig::layer_exposure');
      expect(event.metadata).toMatchObject({
        config: 'test_layer_in_holdout',
        ruleID: 'FC34CQnbBwlkcpMxdi8MT',
        allocatedExperiment: '',
        isExplicitParameter: 'false',
        lcut: anyNumericString(),
        receivedAt: anyNumericString(),
        configVersion: '5',
      });
    });

    it('should contain the expected secondary exposures', async () => {
      const event = events?.[0] as Record<string, any>;
      expect(event?.secondaryExposures).toHaveLength(2);
      expect(event?.secondaryExposures).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            gate: 'global_holdout',
            gateValue: 'false',
            ruleID: '3QoA4ncNdVGBaMt3N1KYjz:0.50:1',
          }),
          expect.objectContaining({
            gate: 'layer_holdout',
            gateValue: 'false',
            ruleID: '2bAVp6R3C85vCYrR6be36n:10.00:5',
          }),
        ]),
      );
    });
  });

  describe('a user in global holdout', () => {
    beforeAll(async () => {
      const userInGlobalHoldout = StatsigUser.withUserID(
        'user_in_global_holdout_a47',
      );
      const {
        layer: layerResult,
        value: valueResult,
        events: eventsResult,
      } = await runTest(userInGlobalHoldout, scrapi);
      layer = layerResult;
      value = valueResult;
      events = eventsResult;
    });

    it('should not be in any group', async () => {
      expect(layer.getGroupName()).toBeNull();
      expect(value).toBe('layer_default');
      expect(layer.getAllocatedExperimentName()).toBeNull();
      expect(layer.getRuleId()).toBe('3Qfj3hxoLuSh1ORaVBVonj');
    });

    it('should log a layer exposure event', async () => {
      expect(events).toHaveLength(1);
      const event = events?.[0] as Record<string, any>;
      expect(event.eventName).toBe('statsig::layer_exposure');

      expect(event.metadata).toMatchObject({
        config: 'test_layer_in_holdout',
        ruleID: '3Qfj3hxoLuSh1ORaVBVonj',
        allocatedExperiment: '',
        isExplicitParameter: 'false',
        parameterName: 'exp_val',
      });
    });

    it('should only contain the one secondary exposure', async () => {
      expect(layer.getSecondaryExposures()).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            gate: 'global_holdout',
            gateValue: 'true',
            ruleID: '3QoA4ncNdVGBaMt3N1KYjz:0.50:1',
          }),
        ]),
      );
    });
  });
});
