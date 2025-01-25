import { __internal__testObservabilityClient as testObservabilityClient } from '../../build/index.js';
import { MockObservabilityClient } from './MockObservabilityClient';

describe('ObservabilityClient', () => {
  let client: MockObservabilityClient;

  beforeEach(async () => {
    client = new MockObservabilityClient();
  });

  it('should call and wait for initialize', async () => {
    const spy = jest.spyOn(client, 'initialize');

    await testObservabilityClient(client, 'init', '', 0, {});

    expect(spy).toHaveBeenCalled();
  });

  it('should call and wait for increment', async () => {
    const spy = jest.spyOn(client, 'increment');

    await testObservabilityClient(client, 'increment', 'my_metric', 1, {
      key: 'a_value',
    });

    expect(spy).toHaveBeenCalledWith('my_metric', 1, { key: 'a_value' });
  });

  it('should call and wait for gauge', async () => {
    const spy = jest.spyOn(client, 'gauge');

    await testObservabilityClient(client, 'gauge', 'my_gauge', 1, {
      key: 'a_value',
    });

    expect(spy).toHaveBeenCalledWith('my_gauge', 1, { key: 'a_value' });
  });

  it('should call and wait for dist', async () => {
    const spy = jest.spyOn(client, 'dist');

    await testObservabilityClient(client, 'dist', 'my_dist', 1, {
      key: 'a_value',
    });

    expect(spy).toHaveBeenCalledWith('my_dist', 1, { key: 'a_value' });
  });
});
