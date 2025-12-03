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
      custom: {
        noneField: undefined,
        aField: 123,
      }
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('c-user');
    expect(event?.user?.customIDs?.myCustomID).toEqual('c-custom-id');
    expect(event?.user?.email).toEqual('c-user@example.com');
    expect(event?.user?.custom.aField).toBe(123);
    expect(event?.user?.custom.noneField).toBeUndefined();
  });

  it('creates users with no userID', async () => {
    const user = new StatsigUser({
      customIDs: {
        myCustomID: 'c-custom-id',
      },
      email: 'c-user@example.com',
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toBeUndefined();
    expect(event?.user?.customIDs?.myCustomID).toEqual('c-custom-id');
    expect(event?.user?.email).toEqual('c-user@example.com');
  });

  it('creates users with no customIDs', async () => {
    const user = new StatsigUser({
      userID: 'c-user',
      email: 'c-user@example.com',
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('c-user');
    expect(event?.user?.customIDs?.myCustomID).toBeUndefined();
    expect(event?.user?.email).toEqual('c-user@example.com');
  });

  it('creates users with an empty user ID when creating a user with no userID or customID', async () => {
    let user = new StatsigUser({
      userID: undefined as any,
      email: 'c-user@example.com',
    });

    statsig.checkGate(user, 'test-gate');

    expect(user).toBeDefined();
    expect(user.userID).toEqual('');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('');
    expect(event?.user?.email).toBe('c-user@example.com');
    expect(event?.user?.customIDs).toBeUndefined();
  });

  it('should not throw when constructed incorrectly', () => {
    expect(() => {
      const user: StatsigUser = {
        userID: undefined as any,
        email: 'c-user@example.com',
      } as any;

      statsig.checkGate(user, 'test-gate');
    }).not.toThrow();
  });

  describe('statsigEnvironment', () => {
    it('logs environment from user object', async () => {
      const user = new StatsigUser({
        userID: 'env-user',
        statsigEnvironment: {
          tier: 'production',
        },
      });

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::gate_exposure');
      expect(event?.user?.userID).toEqual('env-user');
      expect(event?.user?.statsigEnvironment).toBeDefined();
      expect(event?.user?.statsigEnvironment?.tier).toEqual('production');
    });

    it('logs no environment when not provided', async () => {
      const user = new StatsigUser({
        userID: 'env-user-none',
      });

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::gate_exposure');
      expect(event?.user?.statsigEnvironment).toBeUndefined();
    });

    it('logs environment in config exposure events', async () => {
      const user = new StatsigUser({
        userID: 'config-env-user',
        statsigEnvironment: {
          tier: 'staging',
        },
      });

      statsig.getDynamicConfig(user, 'test_email_config');

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::config_exposure');
      expect(event?.user?.statsigEnvironment).toBeDefined();
      expect(event?.user?.statsigEnvironment?.tier).toEqual('staging');
    });

    it('logs environment in layer exposure events', async () => {
      const user = new StatsigUser({
        userID: 'layer-env-user',
        statsigEnvironment: {
          tier: 'development',
        },
      });

      const layer = statsig.getLayer(user, 'test_layer_with_holdout');
      layer.getValue('shared_number_param', 0);

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('statsig::layer_exposure');
      expect(event?.user?.statsigEnvironment).toBeDefined();
      expect(event?.user?.statsigEnvironment?.tier).toEqual('development');
    });

    it('logs environment in custom events', async () => {
      const user = new StatsigUser({
        userID: 'custom-env-user',
        statsigEnvironment: {
          tier: 'testing',
        },
      });

      statsig.logEvent(user, 'custom_event', 'test_value');

      const event = await getLastLoggedEvent();
      expect(event?.eventName).toEqual('custom_event');
      expect(event?.user?.statsigEnvironment).toBeDefined();
      expect(event?.user?.statsigEnvironment?.tier).toEqual('testing');
    });

    it('handles multiple users with different environments', async () => {
      const user1 = new StatsigUser({
        userID: 'multi-user-1',
        statsigEnvironment: { tier: 'production' },
      });

      const user2 = new StatsigUser({
        userID: 'multi-user-2',
        statsigEnvironment: { tier: 'development' },
      });

      const user3 = new StatsigUser({
        userID: 'multi-user-3',
        // no environment
      });

      statsig.checkGate(user1, 'test-gate');
      await statsig.flushEvents();
      const event1 = scrapi.requests[0].body.events.filter(
        (e: any) => e.eventName !== 'statsig::diagnostics',
      )[0];

      scrapi.requests.length = 0;
      statsig.checkGate(user2, 'test-gate');
      await statsig.flushEvents();
      const event2 = scrapi.requests[0].body.events.filter(
        (e: any) => e.eventName !== 'statsig::diagnostics',
      )[0];

      scrapi.requests.length = 0;
      statsig.checkGate(user3, 'test-gate');
      await statsig.flushEvents();
      const event3 = scrapi.requests[0].body.events.filter(
        (e: any) => e.eventName !== 'statsig::diagnostics',
      )[0];

      expect(event1?.user?.statsigEnvironment?.tier).toEqual('production');
      expect(event2?.user?.statsigEnvironment?.tier).toEqual('development');
      expect(event3?.user?.statsigEnvironment).toBeUndefined();
    });

    it('only accepts tier key in statsigEnvironment', async () => {
      // The type enforces only "tier" key is allowed
      const user = new StatsigUser({
        userID: 'typed-user',
        statsigEnvironment: { tier: 'production' },
      });

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.user?.statsigEnvironment?.tier).toEqual('production');
    });

    it('modify fields after creation', async () => {
      const user = new StatsigUser({
        userID: 'modify-user',
        custom: {
          age: 25,
        },
      });

      // Need to use setter to update internal state, merging with existing custom fields
      user.custom = {
        ...(user.custom || {}),
        mutation: "yes"
      };
      
      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.user?.custom?.age).toEqual(25);
      expect(event?.user?.custom?.mutation).toEqual('yes');
    });

    it('modify fields after creation with setter', async () => {
      const user = new StatsigUser({
        userID: 'another-user',
      });

      user.custom = {
        age: 18,
        mutation: "entered",
        isPremium: true,
      };

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.user?.custom?.age).toEqual(18);
      expect(event?.user?.custom?.mutation).toEqual('entered');
      expect(event?.user?.custom?.isPremium).toEqual(true);
    });

    it('attach new custom fields after creation', async () => {
      const user = new StatsigUser({
        userID: 'attach-user',
      });

      // Need to use setter to update internal state, not direct property assignment
      user.custom = {
        mutation: "yes"
      };

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.user?.custom?.mutation).toEqual('yes');
      expect(event?.user?.userID).toEqual('attach-user');
    });
  });

  describe('toJSON', () => {
    it('should work with JSON.stringify', () => {
      const user = new StatsigUser({
        userID: 'test-user',
        email: 'test@example.com',
        ip: '192.168.1.1',
        customIDs: {
          companyID: 'company-123',
          orgID: 'org-456',
        },
        custom: {
          server_tier: 'production',
          is_synthetic: false,
          serviceVersion: 'v1.0.0',
        },
      });

      const stringified = JSON.stringify(user);
      expect(stringified).toBeDefined();
      let parsed: any = JSON.parse(stringified);
      
      expect(parsed).toBeDefined();
      expect(parsed.userID).toBe('test-user');
      expect(parsed.email).toBe('test@example.com');
      expect(parsed.ip).toBe('192.168.1.1');
      expect(parsed.customIDs).toBeDefined();
      expect(parsed.customIDs.companyID).toBe('company-123');
      expect(parsed.customIDs.orgID).toBe('org-456');
      expect(parsed.custom).toBeDefined();
      expect(parsed.custom.server_tier).toBe('production');
      expect(parsed.custom.is_synthetic).toBe(false);
      expect(parsed.custom.serviceVersion).toBe('v1.0.0');
    });
  });
});
