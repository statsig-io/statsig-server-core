import { Statsig, StatsigUser } from '../../build/index.js';

describe('Bad Network Configuration', () => {
  it('should not throw an error', async () => {
    const statsig = new Statsig('', {
      specsUrl: '',
      logEventUrl: '',
    });
    await statsig.initialize();

    const gate = statsig.getFeatureGate(
      StatsigUser.withUserID('a-user'),
      'test_gate',
    );

    expect(gate.value).toEqual(false);

    await statsig.shutdown();
  });
});
