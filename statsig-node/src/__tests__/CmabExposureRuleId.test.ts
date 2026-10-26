import 'jest-extended';
import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

// Node uses the raw experiment path (use_raw_experiment_with_options)
describe('CMAB Exposure Rule ID', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;

  const getLoggedRuleId = async (): Promise<string | undefined> => {
    await statsig.flushEvents();

    const events = scrapi.requests
      .flatMap((r) => r.body?.events ?? [])
      .filter((e: any) => e.eventName === 'statsig::config_exposure');

    expect(events).toHaveLength(1);
    return events[0].metadata.ruleID;
  };

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/dcs_cmab_exposure_rule_id.json',
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

    const options: StatsigOptions = {
      specsUrl: scrapi.getUrlForPath('/v2/download_config_specs'),
      logEventUrl: scrapi.getUrlForPath('/v1/log_event'),
    };

    statsig = new Statsig('secret-cmab-rule-id-test', options);
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

  it('preserves the explore suffix in the automatic exposure', async () => {
    const user = StatsigUser.withUserID('raw-explore-user');

    const experiment = statsig.getExperiment(user, 'cmab_always_explore');

    expect(experiment.ruleID).toEndWith(':explore');
    expect(await getLoggedRuleId()).toEqual(experiment.ruleID);
  });

  it('keeps the exploit rule id bare in the automatic exposure', async () => {
    const user = StatsigUser.withUserID('raw-exploit-user');

    const experiment = statsig.getExperiment(user, 'cmab_never_explore');

    expect(experiment.ruleID).toEqual('treatment-group-id');
    expect(await getLoggedRuleId()).toEqual(experiment.ruleID);
  });
});
