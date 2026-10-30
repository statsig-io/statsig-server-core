import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import * as http from 'node:http';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import * as StableSdk from '../../build/index.js';
import {
  ForwardProxyUserEnrichmentProvider,
  StatsigFPUserBuilder,
} from '../../build/experimental.js';

// @ts-expect-error FP classes must not be exported directly from the stable entry point.
const stableBuilder = StableSdk.StatsigFPUserBuilder;
// @ts-expect-error FP classes must not be exported directly from the stable entry point.
const stableProvider = StableSdk.ForwardProxyUserEnrichmentProvider;

function startServer(
  handler: (req: http.IncomingMessage, res: http.ServerResponse) => void,
): Promise<{ url: string; close: () => Promise<void> }> {
  const server = http.createServer(handler);
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : 0;
      resolve({
        url: `http://127.0.0.1:${port}`,
        close: () =>
          new Promise((closeResolve, closeReject) =>
            server.close((err) => (err ? closeReject(err) : closeResolve())),
          ),
      });
    });
  });
}

describe('Forward proxy user enrichment', () => {
  test('resolves the experimental package subpath for consumers', () => {
    const consumer = mkdtempSync(join(tmpdir(), 'statsig-experimental-'));
    try {
      const scope = join(consumer, 'node_modules', '@statsig');
      mkdirSync(scope, { recursive: true });
      symlinkSync(
        resolve(__dirname, '../../build'),
        join(scope, 'statsig-node-core'),
        'junction',
      );
      // Use Node's real package resolution rather than Jest's module map.
      execFileSync(
        process.execPath,
        ['-e', `
          const assert = require('node:assert/strict');
          const experimental = require('@statsig/statsig-node-core/experimental');
          const stable = require('@statsig/statsig-node-core');
          assert.equal(typeof experimental.StatsigFPUserBuilder, 'function');
          assert.equal(typeof experimental.ForwardProxyUserEnrichmentProvider, 'function');
          assert.equal(experimental.StatsigFPUserBuilder, stable.experimental.StatsigFPUserBuilder);
          assert.equal(typeof stable.StatsigUser, 'function');
          assert.equal(stable.StatsigFPUserBuilder, undefined);
          assert.equal(stable.ForwardProxyUserEnrichmentProvider, undefined);
        `],
        { cwd: consumer, stdio: 'pipe' },
      );
      execFileSync(process.execPath, ['--input-type=module', '-e', `
        import assert from 'node:assert/strict';
        import { StatsigFPUserBuilder } from '@statsig/statsig-node-core/experimental';
        assert.equal(typeof StatsigFPUserBuilder, 'function');
      `], { cwd: consumer, stdio: 'pipe' });
      writeFileSync(join(consumer, 'consumer.ts'), `
        import { StatsigUser } from '@statsig/statsig-node-core';
        import {
          ForwardProxyEnrichmentOptions,
          ForwardProxyUserEnrichmentProvider,
          StatsigFPUserBuilder,
        } from '@statsig/statsig-node-core/experimental';
        // @ts-expect-error Experimental classes cannot be imported from the stable entry point.
        import { StatsigFPUserBuilder as StableBuilder } from '@statsig/statsig-node-core';
        const options: ForwardProxyEnrichmentOptions = { url: 'http://localhost:8000' };
        const provider: ForwardProxyUserEnrichmentProvider =
          ForwardProxyUserEnrichmentProvider.create('secret-test', options);
        const builder: StatsigFPUserBuilder = new StatsigFPUserBuilder(provider, { userID: 'u' });
        const user: Promise<StatsigUser> = builder.build();
      `);
      execFileSync(
        process.execPath,
        [require.resolve('typescript/bin/tsc'), '--noEmit', '--strict', '--skipLibCheck',
          '--target', 'es2020', '--module', 'commonjs', 'consumer.ts'],
        { cwd: consumer, stdio: 'pipe' },
      );
    } finally {
      rmSync(consumer, { recursive: true, force: true });
    }
  });

  test('keeps FP classes in the experimental namespace', () => {
    expect(stableBuilder).toBeUndefined();
    expect(stableProvider).toBeUndefined();
    expect(StableSdk.experimental.StatsigFPUserBuilder).toBe(StatsigFPUserBuilder);
    expect(StableSdk.experimental.ForwardProxyUserEnrichmentProvider).toBe(
      ForwardProxyUserEnrichmentProvider,
    );
  });

  test('returns a StatsigUser on empty membership success', async () => {
    const server = await startServer((_req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ id_lists: {} }));
    });

    try {
      const provider = ForwardProxyUserEnrichmentProvider.create('secret-test', {
        url: server.url,
        timeoutMs: 500,
        retries: 0,
      });
      const user = await new StatsigFPUserBuilder(provider, {
        userID: 'user-1',
      }).build();
      expect(user).toBeInstanceOf(StableSdk.StatsigUser);
      expect(user.userID).toBe('user-1');
    } finally {
      await server.close();
    }
  });

  test('preserves numeric and string custom ID types', async () => {
    const server = await startServer((_req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ id_lists: {} }));
    });
    try {
      const provider = ForwardProxyUserEnrichmentProvider.create('secret-test', {
        url: server.url, timeoutMs: 1000, retries: 0,
      });
      const args = { userID: 'u', customIDs: { companyID: 123, floatID: 12.5, stringID: '00123' } };
      // JavaScript callers can supply numeric IDs, as accepted by the native binding.
      const user = await new StatsigFPUserBuilder(provider, args as any).build();
      const plain = new StableSdk.StatsigUser(args as any);
      expect(user.toJSON()).toEqual(plain.toJSON());
    } finally {
      await server.close();
    }
  });

  test('rejects typed HTTP failures', async () => {
    const server = await startServer((_req, res) => {
      res.writeHead(503, { 'Content-Type': 'application/json' });
      res.end('nope');
    });

    try {
      const provider = ForwardProxyUserEnrichmentProvider.create('secret-test', {
        url: server.url,
        timeoutMs: 500,
        retries: 0,
      });
      await expect(
        new StatsigFPUserBuilder(provider, { userID: 'user-1' }).build(),
      ).rejects.toThrow(/HTTP 503/);
    } finally {
      await server.close();
    }
  });
});
