import { execSync } from 'node:child_process';

import { Statsig } from '../../build/index.js';

function getNumThreads() {
  const pid = process.pid;

  try {
    const threads = execSync(`ps -o nlwp ${pid}`)
      .toString()
      .split('\n')[1]
      .trim();
    console.log(`Process owns ${threads} threads`);
    return threads;
  } catch (err) {
    console.error('Failed to get thread count', err);
  }
}

test('Has correct number of threads', async () => {
  // Baseline includes Node/V8/libuv threads (GC, inspector, libuv threadpool,
  // etc.) which vary by machine, OS, and Node version - that's why this test's
  // absolute threshold has repeatedly been bumped (16 -> 19 -> 20) as CI
  // runners changed. What we actually want to guard against is a regression
  // of the shared tokio runtime (see statsig_global.rs) back into a
  // per-instance runtime, which would scale threads with instance count.
  const baseline = new Statsig('secret-num-threads-test', {
    disableNetwork: true,
  });
  await baseline.initialize();
  const baselineThreads = Number(getNumThreads());

  const instances = [baseline];
  for (let i = 0; i < 9; i++) {
    const statsig = new Statsig('secret-num-threads-test', {
      disableNetwork: true,
    });
    instances.push(statsig);
  }

  await Promise.all(instances.map((statsig) => statsig.initialize()));

  const threads = Number(getNumThreads());
  expect(threads - baselineThreads).toBeLessThanOrEqual(3);
});
