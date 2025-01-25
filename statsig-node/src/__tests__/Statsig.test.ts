import { Statsig, StatsigUser } from '../../build/index.js';

xdescribe('Statsig', () => {
  let statsig: Statsig;

  beforeAll(async () => {
    statsig = new Statsig('secret-IiDuNzovZ5z9x75BEjyZ4Y2evYa94CJ9zNtDViKBVdv');
    await statsig.initialize();
  });

  afterAll(async () => {
    await statsig.shutdown();
  });

  it('should be able to initialize', async () => {
    const user = new StatsigUser('a-user');
    const gate = statsig.checkGate(user, 'a_gate');

    expect(gate).toBe(true);
  });
});
