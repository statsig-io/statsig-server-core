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
  const firstInstance = new Statsig('secret-num-threads-test', {
    disableNetwork: true,
  });
  const instances = [firstInstance];

  try {
    await firstInstance.initialize();
    const threadsAfterFirstInstance = Number(getNumThreads());

    for (let i = 0; i < 9; i++) {
      instances.push(
        new Statsig('secret-num-threads-test', {
          disableNetwork: true,
        }),
      );
    }

    await Promise.all(
      instances.slice(1).map((statsig) => statsig.initialize()),
    );

    const threadsAfterTenInstances = Number(getNumThreads());

    if (
      !Number.isFinite(threadsAfterFirstInstance) ||
      !Number.isFinite(threadsAfterTenInstances)
    ) {
      console.warn(
        'Skipping thread count assertion because the environment does not expose process thread counts',
      );
      return;
    }

    expect(threadsAfterTenInstances).toBeLessThanOrEqual(
      threadsAfterFirstInstance + 1,
    );
  } finally {
    await Promise.all(instances.map((statsig) => statsig.shutdown()));
  }
});
