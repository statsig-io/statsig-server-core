import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('SdkEventEmitterUsage', () => {
  const user = StatsigUser.withUserID('a-user');

  let statsig: Statsig;
  let scrapi: MockScrapi;
  let loggedEvents: any[] = [];

  const pollEvent = async () => {
    const start = Date.now();
    while (loggedEvents.length === 0) {
      await new Promise((r) => setTimeout(r, 1));
      if (Date.now() - start > 1000) {
        throw new Error('Event not logged in 1 second');
      }
    }
    return loggedEvents.shift();
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

    statsig.subscribe('*', (event) => {
      loggedEvents.push(event);
    });
  });

  beforeEach(() => {
    loggedEvents = [];
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('emits gate evaluation events for checkGate', async () => {
    statsig.checkGate(user, 'test_public');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'gate_evaluated',
      gate_name: 'test_public',
      value: true,
      rule_id: '6X3qJgyfwA81IJ2dxI7lYp',
      reason: 'Network:Recognized',
    });
  });

  it('emits gate evaluation events for getFeatureGate', async () => {
    statsig.getFeatureGate(user, 'test_public');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'gate_evaluated',
      gate_name: 'test_public',
      value: true,
      rule_id: '6X3qJgyfwA81IJ2dxI7lYp',
      reason: 'Network:Recognized',
    });
  });

  it('emits dynamic config evaluation events for getDynamicConfig', async () => {
    statsig.getDynamicConfig(user, 'operating_system_config');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'dynamic_config_evaluated',
      dynamic_config: expect.objectContaining({
        name: 'operating_system_config',
        value: {
          num: 13,
          bool: true,
          str: 'hello',
          arr: ['hi', 'there'],
          obj: { a: 'bc' },
        },
        rule_id: 'default',
        details: expect.objectContaining({
          reason: 'Network:Recognized',
        }),
      }),
    });
  });

  it('emits experiment evaluation events for getExperiment', async () => {
    statsig.getExperiment(user, 'exp_with_obj_and_array');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'experiment_evaluated',
      experiment: expect.objectContaining({
        name: 'exp_with_obj_and_array',
        value: {
          arr_param: [true, false, true],
          obj_param: {
            group: 'test',
          },
        },
        rule_id: '23gt15KsgEAbUiwEapclqk',
        details: expect.objectContaining({
          reason: 'Network:Recognized',
        }),
      }),
    });
  });

  it('emits layer evaluation events for getLayer', async () => {
    statsig.getLayer(user, 'layer_with_many_params');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'layer_evaluated',
      layer: expect.objectContaining({
        name: 'layer_with_many_params',
        __value: {
          a_bool: false,
          a_number: 799,
          a_string: 'layer',
          an_array: ['layer_default'],
          an_object: {
            value: 'layer_default',
          },
          another_bool: true,
          another_number: 0,
          another_string: 'layer_default',
        },
        details: expect.objectContaining({
          reason: 'Network:Recognized',
        }),
      }),
    });
  });
});
