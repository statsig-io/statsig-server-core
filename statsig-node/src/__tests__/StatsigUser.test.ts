import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('StatsigUser', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;

  const getLastLoggedEvent = async (): Promise<Record<string, any> | null> => {
    await statsig.flushEvents();

    if (scrapi.requests.length === 0) {
      return null;
    }

    const request = scrapi.requests[0];

    if (!request.body.events) {
      return null;
    }

    const events = request.body.events;
    return (
      events.filter((e: any) => e.eventName !== 'statsig::diagnostics')[0] ??
      null
    );
  };

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-lib/tests/data/eval_proj_dcs.json',
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

  beforeEach(async () => {
    await statsig.flushEvents();
    scrapi.requests.length = 0;
  });

  it('creates users with userID static helper', async () => {
    const user = StatsigUser.withUserID('a-user');
    user.customIDs = {
      myCustomID: 'a-custom-id',
    };
    user.email = 'a-user@example.com';

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('a-user');
    expect(event?.user?.customIDs?.myCustomID).toEqual('a-custom-id');
    expect(event?.user?.email).toEqual('a-user@example.com');
  });

  it('creates users with customIDs static helper', async () => {
    const user = StatsigUser.withCustomIDs({
      myCustomID: 'b-custom-id',
    });
    user.userID = 'b-user';
    user.email = 'b-user@example.com';

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('b-user');
    expect(event?.user?.customIDs?.myCustomID).toEqual('b-custom-id');
    expect(event?.user?.email).toEqual('b-user@example.com');
  });

  it('creates users with constructor', async () => {
    const user = new StatsigUser({
      userID: 'c-user',
      customIDs: {
        myCustomID: 'c-custom-id',
      },
      email: 'c-user@example.com',
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('c-user');
    expect(event?.user?.customIDs?.myCustomID).toEqual('c-custom-id');
    expect(event?.user?.email).toEqual('c-user@example.com');
  });
});
