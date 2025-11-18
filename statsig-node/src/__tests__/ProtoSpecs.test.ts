import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

describe('Proto Specs', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;

  describe('when experimental flag is set', () => {
    beforeAll(async () => {
      const { statsig: newStatsig, scrapi: newScrapi } = await setup(
        new Set(['enable_proto_spec_support']),
      );
      statsig = newStatsig;
      scrapi = newScrapi;
    });

    afterAll(async () => {
      await statsig.shutdown();
      scrapi.close();
    });

    it('makes a request to download the config specs', async () => {
      const request = scrapi.requests[0];
      expect(request.url).toContain(
        '/v2/download_config_specs/secret-123.json?supports_proto=true',
      );
    });

    it('gets correct results from the config specs', async () => {
      const user = StatsigUser.withUserID('a-user');
      const gate = statsig.getFeatureGate(user, 'test_public');
      expect(gate).toEqual({
        name: 'test_public',
        value: true,
        ruleID: '6X3qJgyfwA81IJ2dxI7lYp',
        idType: 'userID',
        details: {
          reason: 'Network:Recognized',
          lcut: expect.any(Number),
          receivedAt: expect.any(Number),
          version: expect.any(Number),
        },
      });
    });
  });

  describe('when experimental flag is not set', () => {
    beforeAll(async () => {
      const { statsig: newStatsig, scrapi: newScrapi } = await setup(new Set());
      statsig = newStatsig;
      scrapi = newScrapi;
    });

    afterAll(async () => {
      await statsig.shutdown();
      scrapi.close();
    });

    it('does not include the supports_proto query param', async () => {
      const request = scrapi.requests[0];
      expect(request.url).not.toContain('supports_proto=');
    });

    it('gets correct results from the config specs', async () => {
      const user = StatsigUser.withUserID('a-user');
      const gate = statsig.getFeatureGate(user, 'test_public');
      expect(gate).toEqual({
        name: 'test_public',
        value: true,
        ruleID: '6X3qJgyfwA81IJ2dxI7lYp',
        idType: 'userID',
        details: {
          reason: 'Network:Recognized',
          lcut: expect.any(Number),
          receivedAt: expect.any(Number),
          version: expect.any(Number),
        },
      });
    });
  });
});

async function setup(
  experimentalFlags: Set<string>,
): Promise<{ statsig: Statsig; scrapi: MockScrapi }> {
  const scrapi = await MockScrapi.create();

  if (experimentalFlags.has('enable_proto_spec_support')) {
    const dcs: Buffer = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/eval_proj_dcs.pb.br',
      ),
    );

    scrapi.mock('/v2/download_config_specs', dcs, {
      status: 200,
      method: 'GET',
      headers: {
        'content-type': 'application/octet-stream',
      },
    });
  } else {
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
  }

  scrapi.mock('/v1/log_event', '{"success": true}', {
    status: 202,
    method: 'POST',
  });

  const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
  const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
  const options: StatsigOptions = {
    specsUrl,
    logEventUrl,
    environment: 'development',
    experimentalFlags,
  };

  const statsig = new Statsig('secret-123', options);
  await statsig.initialize();

  return { statsig, scrapi };
}
