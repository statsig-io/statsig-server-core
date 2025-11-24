import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser, StatsigUserArgs } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

function genUserFromVC(): StatsigUserArgs {
  const user: StatsigUserArgs & {
    custom: Record<string, unknown>;
    customIDs: Record<string, unknown>;
  } = {
    userID: '',
    ip: '192.168.1.1',
    custom: {
      server_tier: 'production',
      is_synthetic: false,
      serviceVersion: 'test-version',
      service: 'test-service',
    },
    customIDs: {
      companyID: 'company-123',
    },
  };

  return user;
}

describe('StatsigUserArgs', () => {
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

  it('create user with statsigUserArgs', async () => {
    const args: StatsigUserArgs = {
      userID: 'a-user',
      customIDs: {
        myCustomID: 'a-custom-id',
      },
      email: 'a-user@example.com',
      ip: '127.0.0.1',
    };
    const user = new StatsigUser({
      ...args,
      userID: args.userID ?? '',
      customIDs: args.customIDs ?? {},
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.userID).toEqual('a-user');
    expect(event?.user?.customIDs?.myCustomID).toEqual('a-custom-id');
    expect(event?.user?.email).toEqual('a-user@example.com');
    expect(event?.user?.ip).toEqual('127.0.0.1');
  });

  it('mutate user with statsigUserArgs', async () => {
    const args: StatsigUserArgs = {
      userID: 'a-user',
      customIDs: {
        myCustomID: 'a-custom-id',
      },
      email: 'whd@statsig.com'
    };
    args.email = 'tore@statsig.com';
    args.custom = {
      ...(args.custom || {}),
      mutation: 'mutation',
    };

    const user = new StatsigUser({
      ...args,
      userID: args.userID ?? '',
      customIDs: args.customIDs ?? {},
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.metadata?.gate).toEqual('test-gate');
    expect(event?.user?.email).toEqual('tore@statsig.com');
    expect(event?.user?.userID).toEqual('a-user');
    expect(event?.user?.customIDs?.myCustomID).toEqual('a-custom-id');
    expect(event?.user?.custom?.mutation).toEqual('mutation');
  });

  it('should attach all custom fields from getBasicUserFromVC', async () => {
    // Simulate what getBasicUserFromVC would return
    const args: StatsigUserArgs = {
      userID: 'user-123',
      ip: '192.168.1.1',
      userAgent: 'Mozilla/5.0',
      custom: {
        server_tier: 'production',
        is_synthetic: false,
        serviceVersion: 'v1.0.0',
        service: 'my-service',
        dagster_job: 'my-job',
        companyID: 'company-789',
        sessionScopeType: 'intern_nonce',
        loginAsCallerEmail: 'admin@example.com',
      },
      customIDs: {
        podID: 'pod-123',
        stableID: 'stable-456',
        companyID: 'company-789',
        orgID: 'org-101',
        mexQueryID: 'query-202',
      },
    };

    const user = new StatsigUser({
      ...args,
      userID: args.userID ?? '',
      customIDs: args.customIDs ?? {},
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    expect(event?.user?.userID).toEqual('user-123');
    expect(event?.user?.ip).toEqual('192.168.1.1');
    expect(event?.user?.userAgent).toEqual('Mozilla/5.0');

    // Verify custom fields are attached
    expect(event?.user?.custom?.server_tier).toEqual('production');
    expect(event?.user?.custom?.is_synthetic).toEqual(false);
    expect(event?.user?.custom?.serviceVersion).toEqual('v1.0.0');
    expect(event?.user?.custom?.service).toEqual('my-service');
    expect(event?.user?.custom?.dagster_job).toEqual('my-job');
    expect(event?.user?.custom?.companyID).toEqual('company-789');
    expect(event?.user?.custom?.sessionScopeType).toEqual('intern_nonce');
    expect(event?.user?.custom?.loginAsCallerEmail).toEqual('admin@example.com');

    // Verify customIDs are attached
    expect(event?.user?.customIDs?.podID).toEqual('pod-123');
    expect(event?.user?.customIDs?.stableID).toEqual('stable-456');
    expect(event?.user?.customIDs?.companyID).toEqual('company-789');
    expect(event?.user?.customIDs?.orgID).toEqual('org-101');
    expect(event?.user?.customIDs?.mexQueryID).toEqual('query-202');
  });

  it('should handle user without optional fields', async () => {
    // Test with minimal fields (like when VC is null or has minimal data)
    const args: StatsigUserArgs = {
      userID: '',
      ip: '10.0.0.1',
      custom: {
        server_tier: 'dev',
        is_synthetic: false,
        serviceVersion: 'v1.0.0',
        service: 'my-service',
      },
      customIDs: {
        podID: 'pod-456',
      },
    };

    const user = new StatsigUser({
      ...args,
      userID: args.userID ?? '',
      customIDs: args.customIDs ?? {},
    });

    statsig.checkGate(user, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.user?.userID).toEqual('');
    expect(event?.user?.ip).toEqual('10.0.0.1');
    expect(event?.user?.custom?.server_tier).toEqual('dev');
    expect(event?.user?.customIDs?.podID).toEqual('pod-456');
    
    // Optional fields should not be present
    expect(event?.user?.custom?.dagster_job).toBeUndefined();
    expect(event?.user?.customIDs?.stableID).toBeUndefined();
    expect(event?.user?.customIDs?.companyID).toBeUndefined();
  });

  it('should work with extended type StatsigUserArgs & { custom: Record<string, unknown>; customIDs: Record<string, unknown>; }', async () => {
    // Test the exact type definition pattern from getBasicUserFromVC
    const user: StatsigUserArgs & {
      custom: Record<string, unknown>;
      customIDs: Record<string, unknown>;
    } = {
      userID: 'user-extended',
      ip: '192.168.1.100',
      userAgent: 'Mozilla/5.0 (Test)',
      custom: {
        server_tier: 'production',
        is_synthetic: false,
        serviceVersion: 'v2.0.0',
        service: 'test-service',
        dagster_job: 'test-job',
        companyID: 'company-123',
        sessionScopeType: 'user',
        loginAsCallerEmail: 'test@example.com',
        // Test that we can add any custom fields
        additionalField: 'test-value',
        nestedObject: { key: 'value' },
      },
      customIDs: {
        podID: 'pod-extended',
        stableID: 'stable-789',
        companyID: 'company-123',
        orgID: 'org-456',
        mexQueryID: 'query-789',
        // Test that we can add any custom IDs
        customID1: 'value1',
        customID2: 'value2',
      },
    };

    const statsigUser = new StatsigUser({
      ...user,
      userID: user.userID ?? '',
      customIDs: user.customIDs ?? {},
    });

    statsig.checkGate(statsigUser, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.eventName).toEqual('statsig::gate_exposure');
    
    // Verify all basic fields
    expect(event?.user?.userID).toEqual('user-extended');
    expect(event?.user?.ip).toEqual('192.168.1.100');
    expect(event?.user?.userAgent).toEqual('Mozilla/5.0 (Test)');

    // Verify all custom fields are attached
    expect(event?.user?.custom?.server_tier).toEqual('production');
    expect(event?.user?.custom?.is_synthetic).toEqual(false);
    expect(event?.user?.custom?.serviceVersion).toEqual('v2.0.0');
    expect(event?.user?.custom?.service).toEqual('test-service');
    expect(event?.user?.custom?.dagster_job).toEqual('test-job');
    expect(event?.user?.custom?.companyID).toEqual('company-123');
    expect(event?.user?.custom?.sessionScopeType).toEqual('user');
    expect(event?.user?.custom?.loginAsCallerEmail).toEqual('test@example.com');
    expect(event?.user?.custom?.additionalField).toEqual('test-value');
    expect(event?.user?.custom?.nestedObject).toEqual({ key: 'value' });

    // Verify all customIDs are attached
    expect(event?.user?.customIDs?.podID).toEqual('pod-extended');
    expect(event?.user?.customIDs?.stableID).toEqual('stable-789');
    expect(event?.user?.customIDs?.companyID).toEqual('company-123');
    expect(event?.user?.customIDs?.orgID).toEqual('org-456');
    expect(event?.user?.customIDs?.mexQueryID).toEqual('query-789');
    expect(event?.user?.customIDs?.customID1).toEqual('value1');
    expect(event?.user?.customIDs?.customID2).toEqual('value2');
  });

  it('should work with extended type and empty initial values', async () => {
    // Test the pattern where custom and customIDs start as empty objects
    const user: StatsigUserArgs & {
      custom: Record<string, unknown>;
      customIDs: Record<string, unknown>;
    } = {
      userID: 'user-empty-init',
      ip: '10.0.0.1',
      custom: {
        server_tier: 'dev',
        is_synthetic: false,
        serviceVersion: 'v1.0.0',
        service: 'my-service',
      },
      customIDs: {},
    };

    // Simulate adding fields conditionally (like in getBasicUserFromVC)
    if (true) { // Simulating a condition
      user.customIDs.podID = 'pod-conditional';
    }

    if (true) { // Simulating another condition
      user.customIDs.stableID = 'stable-conditional';
    }

    if (true) { // Simulating dagster_job condition
      user.custom.dagster_job = 'conditional-job';
    }

    const statsigUser = new StatsigUser({
      ...user,
      userID: user.userID ?? '',
      customIDs: user.customIDs ?? {},
    });

    statsig.checkGate(statsigUser, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.user?.userID).toEqual('user-empty-init');
    expect(event?.user?.custom?.server_tier).toEqual('dev');
    expect(event?.user?.custom?.is_synthetic).toEqual(false);
    expect(event?.user?.custom?.dagster_job).toEqual('conditional-job');
    expect(event?.user?.custom?.serviceVersion).toEqual('v1.0.0');
    expect(event?.user?.custom?.service).toEqual('my-service');
    expect(event?.user?.customIDs?.podID).toEqual('pod-conditional');
    expect(event?.user?.customIDs?.stableID).toEqual('stable-conditional');
  });

  it('should work with extended type when conditions are false (fields not added)', async () => {
    // Test the pattern where conditions are false, so fields are not added
    const user: StatsigUserArgs & {
      custom: Record<string, unknown>;
      customIDs: Record<string, unknown>;
    } = {
      userID: 'user-false-conditions',
      ip: '10.0.0.2',
      custom: {
        server_tier: 'production',
        is_synthetic: false,
        serviceVersion: 'v1.0.0',
        service: 'my-service',
      },
      customIDs: {},
    };

    // Simulate conditions being false (like in getBasicUserFromVC)
    const hasDagsterJob = false; // Simulating env.DAGSTER_RUN_JOB_NAME is undefined
    const isForClient = true; // When true, podID should not be added
    const hasStableID = false; // Simulating stableID is null/undefined
    const hasCompanyID = false; // Simulating companyID is null/undefined
    const hasOrgID = false; // Simulating orgID is null/undefined
    const hasMexQueryID = false; // Simulating mexQueryID is null/undefined

    if (hasDagsterJob) {
      user.custom.dagster_job = 'some-job';
    }

    if (!isForClient) {
      user.customIDs.podID = 'pod-123';
    }

    if (hasStableID) {
      user.customIDs.stableID = 'stable-123';
    }

    if (hasCompanyID) {
      user.customIDs.companyID = 'company-123';
      user.custom.companyID = 'company-123';
    }

    if (hasOrgID) {
      user.customIDs.orgID = 'org-123';
    }

    if (hasMexQueryID) {
      user.customIDs.mexQueryID = 'query-123';
    }

    const statsigUser = new StatsigUser({
      ...user,
      userID: user.userID ?? '',
      customIDs: user.customIDs ?? {},
    });

    statsig.checkGate(statsigUser, 'test-gate');

    const event = await getLastLoggedEvent();
    expect(event?.user?.userID).toEqual('user-false-conditions');
    expect(event?.user?.ip).toEqual('10.0.0.2');
    
    // Verify required fields are present
    expect(event?.user?.custom?.server_tier).toEqual('production');
    expect(event?.user?.custom?.is_synthetic).toEqual(false);
    expect(event?.user?.custom?.serviceVersion).toEqual('v1.0.0');
    expect(event?.user?.custom?.service).toEqual('my-service');
    
    // Verify optional fields are NOT present when conditions are false
    expect(event?.user?.custom?.dagster_job).toBeUndefined();
    expect(event?.user?.customIDs?.podID).toBeUndefined();
    expect(event?.user?.customIDs?.stableID).toBeUndefined();
    expect(event?.user?.customIDs?.companyID).toBeUndefined();
    expect(event?.user?.custom?.companyID).toBeUndefined();
    expect(event?.user?.customIDs?.orgID).toBeUndefined();
    expect(event?.user?.customIDs?.mexQueryID).toBeUndefined();
  });

  describe('genUserFromVC utility function', () => {
    it('should generate basic user structure', async () => {
      const args = genUserFromVC();

      const user = new StatsigUser({
        ...args,
        userID: args.userID ?? '',
        customIDs: args.customIDs ?? {},
      });

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.user?.userID).toEqual('');
      expect(event?.user?.ip).toEqual('192.168.1.1');
      expect(event?.user?.custom?.server_tier).toEqual('production');
      expect(event?.user?.custom?.is_synthetic).toEqual(false);
      expect(event?.user?.custom?.serviceVersion).toEqual('test-version');
      expect(event?.user?.custom?.service).toEqual('test-service');
    });

    it('should allow modifying the returned user object', async () => {
      const args = genUserFromVC();

      if (!args.custom || args.custom === null) {
        args.custom = {};
      }

      if (!args.customIDs || args.customIDs === null) {
        args.customIDs = {};
      }

      
      // Modify fields like in getBasicUserFromVC
      args.userID = 'user-modified';
      args.ip = '10.0.0.1';
      args.custom.dagster_job = 'my-job';
      args.customIDs.podID = 'pod-123';
      args.customIDs.stableID = 'stable-456';

      const user = new StatsigUser({
        ...args,
        userID: args.userID ?? '',
        customIDs: args.customIDs ?? {},
      });

      statsig.checkGate(user, 'test-gate');

      const event = await getLastLoggedEvent();
      expect(event?.user?.userID).toEqual('user-modified');
      expect(event?.user?.ip).toEqual('10.0.0.1');
      expect(event?.user?.custom?.server_tier).toEqual('production');
      expect(event?.user?.custom?.is_synthetic).toEqual(false);
      expect(event?.user?.custom?.serviceVersion).toEqual('test-version');
      expect(event?.user?.custom?.service).toEqual('test-service');
      expect(event?.user?.custom?.dagster_job).toEqual('my-job');
      expect(event?.user?.customIDs?.podID).toEqual('pod-123');
      expect(event?.user?.customIDs?.stableID).toEqual('stable-456');
      expect(event?.user?.customIDs?.companyID).toEqual('company-123');
    });
  });
});
