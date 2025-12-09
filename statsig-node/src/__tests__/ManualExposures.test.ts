import 'jest-extended';
import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi, anyNumericString } from './MockScrapi';

const KNOWN_EXP_PARAM = 'exp_val';
const KNOWN_DEFAULT_PARAM = 'layer_val';

const KNOWN_SEC_EXPO_GLOBAL_HOLDOUT = {
  gate: 'global_holdout',
  gateValue: 'false',
  ruleID: '3QoA4ncNdVGBaMt3N1KYjz:0.50:1',
};

const KNOWN_SEC_EXPO_LAYER_HOLDOUT = {
  gate: 'layer_holdout',
  gateValue: 'false',
  ruleID: '2bAVp6R3C85vCYrR6be36n:10.00:5',
};

const KNOWN_SEC_EXPO_TARGETING = {
  gate: 'test_50_50',
  gateValue: 'true',
  ruleID: '6U5gYSQ2jRCDWvfPzKSQY9',
};

describe('Manual Exposures', () => {
  const userInLayerTest = StatsigUser.withUserID('user-d');
  const userInLayerControl = StatsigUser.withUserID('user-b');

  let statsig: Statsig;
  let scrapi: MockScrapi;

  const getLoggedEvents = async (): Promise<Record<string, any> | null> => {
    await statsig.flushEvents();

    if (scrapi.requests.length === 0) {
      return null;
    }

    const request = scrapi.requests[0];

    if (!request?.body?.events) {
      return null;
    }

    return request.body.events.filter(
      (e: any) => e.eventName !== 'statsig::diagnostics',
    );
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

  beforeEach(async () => {
    await statsig.flushEvents();
    scrapi.requests.length = 0;
  });

  it('should not log exposures when disabled', async () => {
    const noExpo = { disableExposureLogging: true };

    statsig.getFeatureGate(userInLayerTest, 'test_public', noExpo);
    statsig.checkGate(userInLayerTest, 'test_public', noExpo);
    statsig.getDynamicConfig(userInLayerTest, 'big_number', noExpo);
    statsig.getExperiment(
      userInLayerTest,
      'test_experiment_no_targeting',
      noExpo,
    );

    const layer = statsig.getLayer(
      userInLayerTest,
      'test_layer_in_holdout',
      noExpo,
    );
    layer.getValue(KNOWN_EXP_PARAM, 'err');
    layer.getValue(KNOWN_DEFAULT_PARAM, 'err');

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::non_exposed_checks');
    expect(JSON.parse(events?.[0].metadata.checks)).toEqual({
      test_layer_in_holdout: 3, // get layer then get param
      big_number: 1,
      test_public: 2, // checkGate and getFeatureGate
      test_experiment_no_targeting: 1,
    });
  });

  it('can manually log gate exposures', async () => {
    statsig.manuallyLogFeatureGateExposure(userInLayerTest, 'test_public');

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::gate_exposure');
    expect(events?.[0].metadata).toMatchObject({
      gate: 'test_public',
      gateValue: 'true',
      ruleID: '6X3qJgyfwA81IJ2dxI7lYp',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
    });
  });

  it('can manually log dynamic config exposures', async () => {
    statsig.manuallyLogDynamicConfigExposure(userInLayerTest, 'big_number');
    const events = await getLoggedEvents();
    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::config_exposure');
    expect(events?.[0].metadata).toMatchObject({
      config: 'big_number',
      ruleID: 'default',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
    });
  });

  it('can manually log experiment exposures', async () => {
    statsig.manuallyLogExperimentExposure(
      userInLayerTest,
      'test_experiment_no_targeting',
    );

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::config_exposure');
    expect(events?.[0].metadata).toMatchObject({
      config: 'test_experiment_no_targeting',
      ruleID: '54QJztEPRLXK7ZCvXeY9q4',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
    });
  });

  it('can manually log layer explicit param exposures for a test user', async () => {
    statsig.manuallyLogLayerParamExposure(
      userInLayerTest,
      'test_layer_in_holdout',
      KNOWN_EXP_PARAM,
    );

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::layer_exposure');
    expect(events?.[0].metadata).toMatchObject({
      config: 'test_layer_in_holdout',
      ruleID: '6h3c5rGSyHu4p1sbIBqltg',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
      parameterName: KNOWN_EXP_PARAM,
      isExplicitParameter: 'true',
      allocatedExperiment: 'targeted_exp_in_layer_with_holdout',
    });

    // duplicated as all heck
    expect(events?.[0].secondaryExposures).toEqual([
      KNOWN_SEC_EXPO_GLOBAL_HOLDOUT,
      KNOWN_SEC_EXPO_LAYER_HOLDOUT,
      KNOWN_SEC_EXPO_GLOBAL_HOLDOUT,
      KNOWN_SEC_EXPO_LAYER_HOLDOUT,
      KNOWN_SEC_EXPO_GLOBAL_HOLDOUT,
      KNOWN_SEC_EXPO_TARGETING,
    ]);
  });

  it('can manually log layer explicit param exposures for a control user', async () => {
    statsig.manuallyLogLayerParamExposure(
      userInLayerControl,
      'test_layer_in_holdout',
      KNOWN_EXP_PARAM,
    );

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::layer_exposure');
    expect(events?.[0].metadata).toMatchObject({
      config: 'test_layer_in_holdout',
      ruleID: 'FC34BbkJjmR0MRQW5mvtR',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
      parameterName: KNOWN_EXP_PARAM,
      isExplicitParameter: 'true',
      allocatedExperiment: 'running_exp_in_layer_with_holdout',
    });

    expect(events?.[0].secondaryExposures).toEqual([
      KNOWN_SEC_EXPO_GLOBAL_HOLDOUT,
      KNOWN_SEC_EXPO_LAYER_HOLDOUT,
      KNOWN_SEC_EXPO_GLOBAL_HOLDOUT,
      KNOWN_SEC_EXPO_LAYER_HOLDOUT,
    ]);
  });

  it('can manually log layer default param exposures for a test user', async () => {
    statsig.manuallyLogLayerParamExposure(
      userInLayerTest,
      'test_layer_in_holdout',
      KNOWN_DEFAULT_PARAM,
    );

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::layer_exposure');
    expect(events?.[0].metadata).toMatchObject({
      config: 'test_layer_in_holdout',
      ruleID: '6h3c5rGSyHu4p1sbIBqltg',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
      parameterName: KNOWN_DEFAULT_PARAM,
      isExplicitParameter: 'false',
      allocatedExperiment: '',
    });
  });

  it('can manually log layer default param exposures for a control user', async () => {
    statsig.manuallyLogLayerParamExposure(
      userInLayerControl,
      'test_layer_in_holdout',
      KNOWN_DEFAULT_PARAM,
    );

    const events = await getLoggedEvents();

    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::layer_exposure');
    expect(events?.[0].metadata).toMatchObject({
      config: 'test_layer_in_holdout',
      ruleID: 'FC34BbkJjmR0MRQW5mvtR',
      reason: 'Network:Recognized',
      lcut: anyNumericString(),
      receivedAt: anyNumericString(),
      configVersion: anyNumericString(),
      isManualExposure: 'true',
      parameterName: KNOWN_DEFAULT_PARAM,
      isExplicitParameter: 'false',
      allocatedExperiment: '',
    });
  });
});
