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
    const result = statsig.getExperimentGroups('test_experiment_no_targeting');

    expect(result.isExperimentActive).toBe(true);

    const groupsByName = Object.fromEntries(
      result.groups.map((group) => [group.groupName, group]),
    );

    // Only the experiment group rules are returned (the layerAssignment rule is excluded).
    expect(Object.keys(groupsByName).sort()).toEqual(['Control', 'Test', 'Test2']);
    expect(groupsByName['Control'].returnValue).toEqual({ value: 'control' });
    expect(groupsByName['Control'].ruleId).toBe('54QJztEPRLXK7ZCvXeY9q4');
    expect(groupsByName['Control'].idType).toBe('userID');
    expect(groupsByName['Test'].returnValue).toEqual({ value: 'test_1' });
    expect(groupsByName['Test2'].returnValue).toEqual({ value: 'test_2' });
  });

  it('should return undefined active state for an unknown experiment', async () => {
    const result = statsig.getExperimentGroups('not_an_experiment');

    expect(result.isExperimentActive).toBeUndefined();
    expect(result.groups).toEqual([]);
  });

  it('should return undefined active state for a dynamic config', async () => {
    const result = statsig.getExperimentGroups(
      'test_max_dynamic_config_size_again',
    );

    expect(result.isExperimentActive).toBeUndefined();
    expect(result.groups).toEqual([]);
  });

  it('should return the groups for an inactive experiment', async () => {
    // test_switchback has isActive: false; groups are still returned along with the flag.
    const result = statsig.getExperimentGroups('test_switchback');

    expect(result.isExperimentActive).toBe(false);

    // Only the experiment group rules are returned (non-group rules are excluded).
    const groupNames = result.groups.map((group) => group.groupName).sort();
    expect(groupNames).toEqual(['Control', 'Test']);
  });
});
