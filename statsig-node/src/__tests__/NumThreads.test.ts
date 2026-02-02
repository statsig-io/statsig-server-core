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
  const instances = [];
  for (let i = 0; i < 10; i++) {
    const statsig = new Statsig('secret-num-threads-test', {
      disableNetwork: true,
    });
    instances.push(statsig);
  }

  await Promise.all(instances.map((statsig) => statsig.initialize()));

  const threads = Number(getNumThreads());
  expect(threads).toBeLessThan(19);
});
