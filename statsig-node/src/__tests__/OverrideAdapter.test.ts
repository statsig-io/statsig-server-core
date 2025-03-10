import * as fs from 'node:fs';
import * as path from 'node:path';

import {
  OverrideAdapterType,
  Statsig,
  StatsigOptions,
  StatsigUser,
} from '../../build';
import { MockScrapi } from './MockScrapi';

describe('Override Adapter Usage', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;
  let options: StatsigOptions;

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
    options = { specsUrl, logEventUrl };
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  const testCases = [
    {
      title: 'OverrideAdapter defaults to LocalOverride',
      optionsModifier: (opts: StatsigOptions) => opts,
    },
    {
      title: 'OverrideAdapter with Default Adapter Config',
      optionsModifier: (opts: StatsigOptions) => ({
        ...opts,
        overrideAdapterConfig: [
          {
            adapterType: OverrideAdapterType.LocalOverride,
          },
        ],
      }),
    },
  ];

  testCases.forEach(({ title, optionsModifier }) => {
    describe(title, () => {
      beforeAll(async () => {
        statsig = new Statsig('secret-123', optionsModifier(options));
        await statsig.initialize();
      });

      const user = StatsigUser.withUserID('a-user');

      const overrideTests = [
        {
          name: 'should override gates',
          action: () => statsig.overrideGate('test_public', false),
          getter: () => statsig.getFeatureGate(user, 'test_public'),
          expectedValue: false,
        },
        {
          name: 'should override dynamic configs',
          action: () =>
            statsig.overrideDynamicConfig('operating_system_config', {
              foo: 'bar',
            }),
          getter: () =>
            statsig.getDynamicConfig(user, 'operating_system_config'),
          expectedValue: { foo: 'bar' },
        },
        {
          name: 'should override experiments',
          action: () =>
            statsig.overrideExperiment('test_exp_random_id', { foo: 'bar' }),
          getter: () => statsig.getExperiment(user, 'test_exp_random_id'),
          expectedValue: { foo: 'bar' },
        },
        {
          name: 'should override experiments by group name',
          action: () =>
            statsig.overrideExperimentByGroupName(
              'test_exp_random_id',
              'Control',
            ),
          getter: () => statsig.getExperiment(user, 'test_exp_random_id'),
          expectedValue: { bool: false, string: 'control', num: 999 },
        },
        {
          name: 'should override layers',
          action: () => statsig.overrideLayer('test_layer', { foo: 'bar' }),
          getter: () => statsig.getLayer(user, 'test_layer').get('foo', 'err'),
          expectedValue: 'bar',
        },
      ];

      overrideTests.forEach(({ name, action, getter, expectedValue }) => {
        it(name, () => {
          action();
          const result = getter();
          if (name === 'should override layers') {
            const layer = statsig.getLayer(user, 'test_layer');
            expect(layer.get('foo', 'err')).toEqual(expectedValue);
            expect(layer.ruleID).toBe('override');
          } else {
            expect(result.value).toEqual(expectedValue);
            expect(result.ruleID).toBe('override');
          }
        });
      });
    });
  });
});
