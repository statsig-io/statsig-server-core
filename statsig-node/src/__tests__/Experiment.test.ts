import * as fs from 'node:fs';
import * as path from 'node:path';

import { DynamicConfig, Statsig, StatsigOptions, StatsigUser } from '../../build';
import { MockScrapi } from './MockScrapi';

describe('Experiment', () => {
  let statsig: Statsig;
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
    const options: StatsigOptions = { specsUrl, logEventUrl };

    statsig = new Statsig('secret-123', options);
    await statsig.initialize();
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it('converts Experiment to DynamicConfig with the right value', () => {
    const user = StatsigUser.withUserID('a-user');
    const experiment = statsig.getExperiment(user, 'exp_with_obj_and_array');

    // Convert Experiment -> DynamicConfig
    const expDetails = experiment.getEvaluationDetails();
    const rawForDynamicConfig = JSON.stringify({
      value: experiment.value,
      ruleID: experiment.getRuleId(),
      idType: experiment.getIdType(),
      details: {
        reason: expDetails.reason,
        lcut: expDetails.lcut,
        received_at: expDetails.receivedAt,
        version: expDetails.version,
      },
      secondaryExposures: experiment.getSecondaryExposures(),
    });
    const dynamicConfig = new DynamicConfig(experiment.name, rawForDynamicConfig);
    expect(dynamicConfig).toBeInstanceOf(DynamicConfig);
    expect(dynamicConfig.name).toEqual(experiment.name);
    expect(dynamicConfig.name).toEqual('exp_with_obj_and_array');

    expect(dynamicConfig.value).toEqual(experiment.value);
    expect(dynamicConfig.get('arr_param', [])).toEqual([true, false, true]);
    expect(dynamicConfig.get('obj_param', {})).toEqual({ group: 'test' });

    expect(dynamicConfig.getRuleId()).toEqual(experiment.getRuleId());
    expect(dynamicConfig.getIdType()).toEqual(experiment.getIdType());
    expect(dynamicConfig.getEvaluationDetails()).toEqual(
      experiment.getEvaluationDetails(),
    );
  });
});
