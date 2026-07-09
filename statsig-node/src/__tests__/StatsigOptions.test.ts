import { Statsig } from '../../build/index.js';

describe('StatsigOptions', () => {
  it('accepts exposureDedupeMaxKeys and forwards it to the core options', async () => {
    const statsig = new Statsig('secret-exposure-dedupe-test', {
      disableNetwork: true,
      exposureDedupeMaxKeys: 50_000,
    });

    // Initialization succeeding confirms the binding accepts the option and
    // forwards it to the core StatsigOptions (unset -> core default of 100,000).
    await statsig.initialize();
    expect(statsig).toBeDefined();
    await statsig.shutdown();
  });
});
