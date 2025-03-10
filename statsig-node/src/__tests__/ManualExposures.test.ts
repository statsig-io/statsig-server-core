import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Manual Exposures', () => {
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
    const user = StatsigUser.withUserID('a-user');
    const noExpo = { disableExposureLogging: true };
    statsig.getFeatureGate(user, 'test_public', noExpo);
    statsig.checkGate(user, 'test_public', noExpo);
    statsig.getDynamicConfig(user, 'big_number', noExpo);
    statsig.getExperiment(user, 'test_experiment_no_targeting', noExpo);
    const layer = statsig.getLayer(user, 'layer_with_many_params', noExpo);
    layer.get('another_string', 'err');

    const events = await getLoggedEvents();
    expect(events).toHaveLength(1);
    expect(events?.[0].eventName).toEqual('statsig::non_exposed_checks');
    expect(JSON.parse(events?.[0].metadata.checks)).toEqual({
      layer_with_many_params: 2, // get layer then get param
      big_number: 1,
      test_public: 2, // checkGate and getFeatureGate
      test_experiment_no_targeting: 1,
    });
  });

  it('can manually log exposures', async () => {
    const user = StatsigUser.withUserID('a-user');

    statsig.manuallyLogFeatureGateExposure(user, 'test_public');
    statsig.manuallyLogDynamicConfigExposure(user, 'big_number');
    statsig.manuallyLogExperimentExposure(user, 'test_experiment_no_targeting');
    statsig.manuallyLogLayerParamExposure(
      user,
      'layer_with_many_params',
      'another_string',
    );

    const events = await getLoggedEvents();
    expect(events).toHaveLength(4);
    expect(events?.[0].eventName).toEqual('statsig::gate_exposure');
    expect(events?.[1].eventName).toEqual('statsig::config_exposure');
    expect(events?.[2].eventName).toEqual('statsig::config_exposure');
    expect(events?.[3].eventName).toEqual('statsig::layer_exposure');
  });
});
