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
      beforeEach(async () => {
        statsig = new Statsig('secret-123', optionsModifier(options));
        await statsig.initialize();
      });

      afterEach(async () => {
        await statsig.shutdown();
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
          getter: () => statsig.getLayer(user, 'test_layer').getValue('foo', 'err'),
          expectedValue: 'bar',
        },
        {
          name: 'should override gates for user ID',
          action: () => statsig.overrideGateForId('test_public', 'a-user', false),
          getter: () => statsig.getFeatureGate(user, 'test_public'),
          expectedValue: false,
        },
        {
          name: 'should override dynamic configs for user ID',
          action: () =>
            statsig.overrideDynamicConfigForId('operating_system_config', 'a-user', {
              foo: 'bar',
            }),
          getter: () =>
            statsig.getDynamicConfig(user, 'operating_system_config'),
          expectedValue: { foo: 'bar' },
        },
        {
          name: 'should override experiments for user ID',
          action: () =>
            statsig.overrideExperimentForId('test_exp_random_id', 'a-user', { foo: 'bar' }),
          getter: () => statsig.getExperiment(user, 'test_exp_random_id'),
          expectedValue: { foo: 'bar' },
        },
        {
          name: 'should override experiments by group name for user ID',
          action: () =>
            statsig.overrideExperimentByGroupNameForId(
              'test_exp_random_id',
              'a-user',
              'Control',
            ),
          getter: () => statsig.getExperiment(user, 'test_exp_random_id'),
          expectedValue: { bool: false, string: 'control', num: 999 },
        },
        {
          name: 'should override layers for user ID',
          action: () => statsig.overrideLayerForId('test_layer','a-user', { foo: 'bar' }),
          getter: () =>
            statsig.getLayer(user, 'test_layer'),
          expectedValue: 'bar',
        },
        {
          name: 'should prioritize ID override over name override',
          action: () => {
            statsig.overrideGate('test_public', false);
            statsig.overrideGateForId('test_public', 'a-user', true);
          },
          getter: () => statsig.getFeatureGate(user, 'test_public'),
          expectedValue: true,
        },
        {
          name: 'should override gates for custom ID',
          action: () => {
            const userWithCustomId = StatsigUser.withUserID('custom-user');
            userWithCustomId.customIDs = { employee_id: 'employee_id:12345' };
            statsig.overrideGateForId('test_public', 'employee_id:12345', false);
            return userWithCustomId;
          },
          getter: (customUser: StatsigUser) => statsig.getFeatureGate(customUser, 'test_public'),
          expectedValue: false,
        },
        {
          name: 'should override dynamic configs for custom ID',
          action: () => {
            const userWithCustomId = StatsigUser.withUserID('custom-user');
            userWithCustomId.customIDs = { employee_id: 'employee_id:12345' };
            statsig.overrideDynamicConfigForId('operating_system_config', 'employee_id:12345', { foo: 'bar' });
            return userWithCustomId;
          },
          getter: (customUser: StatsigUser) => statsig.getDynamicConfig(customUser, 'operating_system_config'),
          expectedValue: { foo: 'bar' },
        },
        {
          name: 'should override experiments for custom ID',
          action: () => {
            const userWithCustomId = StatsigUser.withUserID('custom-user');
            userWithCustomId.customIDs = { employee_id: 'employee_id:12345' };
            statsig.overrideExperimentForId('test_exp_random_id', 'employee_id:12345', { foo: 'custom_id_value' });
            return userWithCustomId;
          },
          getter: (customUser: StatsigUser) => statsig.getExperiment(customUser, 'test_exp_random_id'),
          expectedValue: { foo: 'custom_id_value' },
        },
        {
          name: 'should override experiments by group name for custom ID',
          action: () => {
            const userWithCustomId = StatsigUser.withUserID('custom-user');
            userWithCustomId.customIDs = { employee_id: 'employee_id:12345' };
            statsig.overrideExperimentByGroupNameForId('test_exp_random_id', 'employee_id:12345', 'Control');
            return userWithCustomId;
          },
          getter: (customUser: StatsigUser) => statsig.getExperiment(customUser, 'test_exp_random_id'),
          expectedValue: { bool: false, string: 'control', num: 999 },
        },
        {
          name: 'should override layers for custom ID',
          action: () => {
            const userWithCustomId = StatsigUser.withUserID('custom-user');
            userWithCustomId.customIDs = { employee_id: 'employee_id:12345' };
            statsig.overrideLayerForId('test_layer', 'employee_id:12345', { foo: 'custom_id_value' });
            return userWithCustomId;
          },
          getter: (customUser: StatsigUser) => statsig.getLayer(customUser, 'test_layer'),
          expectedValue: 'custom_id_value',
        },
        {
          name: 'should prioritize user ID override over custom ID override',
          action: () => {
            const userWithCustomId = StatsigUser.withUserID('custom-user');
            userWithCustomId.customIDs = { employee_id: '12345' };
            statsig.overrideGate('test_public', false);
            statsig.overrideGateForId('test_public', 'employee_id:12345', true);
            statsig.overrideGateForId('test_public', 'custom-user', false);
            return userWithCustomId;
          },
          getter: (customUser: StatsigUser) => statsig.getFeatureGate(customUser, 'test_public'),
          expectedValue: false,
        },
      ];

      overrideTests.forEach(({ name, action, getter, expectedValue }) => {
        it(name, () => {
          const customUser = action();
          const result = customUser !== undefined ? 
            (getter as (customUser: any) => any)(customUser) : 
            (getter as () => any)();
          if (name.includes('should override layers')) {
            const layer = customUser 
              ? statsig.getLayer(customUser, 'test_layer') 
              : statsig.getLayer(user, 'test_layer');
            expect(layer.getValue('foo', 'err')).toEqual(expectedValue);
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
