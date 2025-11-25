import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

function createStatsigInstance(scrapi: MockScrapi) {
  const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
  const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
  const options: StatsigOptions = {
    specsUrl,
    logEventUrl,
  };

  return new Statsig('secret-123', options);
}

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

    statsig = createStatsigInstance(scrapi);
    await statsig.initialize();

    statsig.subscribe('*', (event) => loggedEvents.push(event));
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
      data: {
        gate_name: 'test_public',
        value: true,
        rule_id: '6X3qJgyfwA81IJ2dxI7lYp',
        reason: 'Network:Recognized',
      },
    });
  });

  it('emits gate evaluation events for getFeatureGate', async () => {
    statsig.getFeatureGate(user, 'test_public');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'gate_evaluated',
      data: {
        gate_name: 'test_public',
        value: true,
        rule_id: '6X3qJgyfwA81IJ2dxI7lYp',
        reason: 'Network:Recognized',
      },
    });
  });

  it('emits dynamic config evaluation events for getDynamicConfig', async () => {
    statsig.getDynamicConfig(user, 'operating_system_config');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'dynamic_config_evaluated',
      data: {
        config_name: 'operating_system_config',
        reason: 'Network:Recognized',
        rule_id: 'default',
        value: expect.objectContaining({
          num: 13,
          bool: true,
          str: 'hello',
          arr: ['hi', 'there'],
          obj: { a: 'bc' },
        }),
      },
    });
  });

  it('emits experiment evaluation events for getExperiment', async () => {
    statsig.getExperiment(user, 'exp_with_obj_and_array');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'experiment_evaluated',
      data: {
        experiment_name: 'exp_with_obj_and_array',
        rule_id: '23gt15KsgEAbUiwEapclqk',
        reason: 'Network:Recognized',
        value: expect.objectContaining({
          arr_param: [true, false, true],
          obj_param: {
            group: 'test',
          },
        }),
      },
    });
  });

  it('emits layer evaluation events for getLayer', async () => {
    statsig.getLayer(user, 'layer_with_many_params');
    const event = await pollEvent();

    expect(event).toEqual({
      event_name: 'layer_evaluated',
      data: {
        layer_name: 'layer_with_many_params',
        reason: 'Network:Recognized',
        rule_id: 'default',
      },
    });
  });

  it('emits specs updated events for initialize', async () => {
    const anotherStatsig = createStatsigInstance(scrapi);

    const events: any[] = [];
    anotherStatsig.subscribe('specs_updated', (event) => events.push(event));
    await anotherStatsig.initialize();

    expect(events).toEqual([
      {
        event_name: 'specs_updated',
        data: {
          source: 'Network',
          source_api: expect.stringContaining('http://localhost'),
          values: expect.objectContaining({
            time: expect.any(Number),
          }),
        },
      },
    ]);
  });
});
