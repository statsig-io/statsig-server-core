import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Get Experiment By Group Id Advanced', () => {
  let scrapi: MockScrapi;
  let statsig: Statsig;

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

  it('should return an experiment by group id', async () => {
    const experiment = statsig.getExperimentByGroupIdAdvanced(
      'test_experiment_no_targeting',
      '54QJztEPRLXK7ZCvXeY9q4',
    );

    expect(experiment).toMatchObject({
      details: {
        lcut: expect.any(Number),
        reason: 'Network:Recognized',
        receivedAt: expect.any(Number),
      },
      groupName: 'Control',
      idType: 'userID',
      name: 'test_experiment_no_targeting',
      ruleID: '54QJztEPRLXK7ZCvXeY9q4',
      value: { value: 'control' },
    });
  });

  it('should return an Experiment with empty groupName/ruleID for an unrecognized experiment', async () => {
    const experiment = statsig.getExperimentByGroupIdAdvanced(
      'not_an_experiment',
      '54QJztEPRLXK7ZCvXeY9q4',
    );

    expect(experiment).toMatchObject({
      details: {
        lcut: expect.any(Number),
        reason: 'Network:Unrecognized',
        receivedAt: expect.any(Number),
      },
      groupName: null,
      idType: '',
      name: 'not_an_experiment',
      ruleID: '',
      value: {},
    });
  });

  it('should return an Experiment with empty groupName/ruleID for an unrecognized group id', async () => {
    const experiment = statsig.getExperimentByGroupIdAdvanced(
      'test_experiment_no_targeting',
      'not_a_group_id',
    );

    expect(experiment).toMatchObject({
      details: {
        lcut: expect.any(Number),
        reason: 'Network:Unrecognized',
        receivedAt: expect.any(Number),
      },
      groupName: null,
      idType: '',
      name: 'test_experiment_no_targeting',
      ruleID: '',
      value: {},
    });
  });
});
