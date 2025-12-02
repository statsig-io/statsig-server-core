import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Get Experiment By Group Name', () => {
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

  it('should return an experiment by Control group name', async () => {
    const experiment = statsig.getExperimentByGroupName(
      'test_experiment_no_targeting',
      'Control',
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

  it('should return an experiment by Test group name', async () => {
    const experiment = statsig.getExperimentByGroupName(
      'test_experiment_no_targeting',
      'Test',
    );

    expect(experiment).toMatchObject({
      details: {
        lcut: expect.any(Number),
        reason: 'Network:Recognized',
        receivedAt: expect.any(Number),
      },
      groupName: 'Test',
      idType: 'userID',
      name: 'test_experiment_no_targeting',
      ruleID: '54QJzvjSk47erparymTMJ6',
      value: { value: 'test_1' },
    });
  });

  it('should return null if the experiment is not found', async () => {
    const experiment = statsig.getExperimentByGroupName(
      'not_an_experiment',
      'Control',
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
      ruleID: 'default',
      value: {},
    });
  });

  it('should return null if the group name is not found', async () => {
    const experiment = statsig.getExperimentByGroupName(
      'test_experiment_no_targeting',
      'InvalidGroupName',
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
      ruleID: 'default',
      value: {},
    });
  });
});
