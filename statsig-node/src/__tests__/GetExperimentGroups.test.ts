import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Get Experiment Groups', () => {
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

  it('should return the groups for a known experiment', async () => {
    const groups = statsig.getExperimentGroups('test_experiment_no_targeting');

    const groupsByName = Object.fromEntries(
      groups.map((group) => [group.groupName, group.returnValue]),
    );

    // Only the experiment group rules are returned (the layerAssignment rule is excluded).
    expect(Object.keys(groupsByName).sort()).toEqual(['Control', 'Test', 'Test2']);
    expect(groupsByName['Control']).toEqual({ value: 'control' });
    expect(groupsByName['Test']).toEqual({ value: 'test_1' });
    expect(groupsByName['Test2']).toEqual({ value: 'test_2' });
  });

  it('should return an empty list for an unknown experiment', async () => {
    const groups = statsig.getExperimentGroups('not_an_experiment');

    expect(groups).toEqual([]);
  });

  it('should return an empty list for a dynamic config', async () => {
    const groups = statsig.getExperimentGroups(
      'test_max_dynamic_config_size_again',
    );

    expect(groups).toEqual([]);
  });

  it('should return an empty list for an inactive experiment', async () => {
    const groups = statsig.getExperimentGroups('an_experiment1');

    expect(groups).toEqual([]);
  });
});
