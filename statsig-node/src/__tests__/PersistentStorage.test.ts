import {
  Statsig,
  __internal__testPersistentStorage as testPersistentStorage,
} from '../../build/index.js';
import { MockPersistentStorage } from './MockPersistentStorage';

const TEST_STICKY_VALUES = {
  test: {
    value: true,
    json_value: {
      header_text: 'old user test',
    },
    rule_id: '3ZCniK9rvnQyXDQlQ1tGD9',
    group_name: 'test_group',
    secondary_exposures: [
      {
        gate: 'test_holdout',
        gateValue: 'true',
        ruleID: 'default',
      },
    ],
    undelegated_secondary_exposures: [],
    config_version: 1,
    time: Date.now(),
    config_delegate: null,
    explicit_parameters: null,
  },
};

describe('PersistentStorage', () => {
  let persistentStorage: MockPersistentStorage;
  let statsig: Statsig;

  beforeAll(() => {
    statsig = new Statsig('secret-123', { outputLogLevel: 'debug' });
  });

  afterAll(() => {
    statsig.shutdown();
  });

  beforeEach(async () => {
    persistentStorage = new MockPersistentStorage();
  });

  it('should load values from the persistent storage', async () => {
    persistentStorage.load_mock.mockReturnValue(TEST_STICKY_VALUES);

    const result = await testPersistentStorage(persistentStorage, 'load');
    const parsed = Object.entries(result ?? {}).reduce((acc, [key, value]) => {
      acc[key] = JSON.parse(value);
      return acc;
    }, {} as any);

    expect(persistentStorage.load_mock).toHaveBeenCalled();
    expect(parsed).toEqual(TEST_STICKY_VALUES);
  });

  it('should save values to the persistent storage', async () => {
    await testPersistentStorage(
      persistentStorage,
      'save',
      'a_key',
      'my_config',
      TEST_STICKY_VALUES.test,
    );

    expect(persistentStorage.save_mock).toHaveBeenCalledWith(
      'a_key',
      'my_config',
      TEST_STICKY_VALUES.test,
    );
  });

  it('should delete values from the persistent storage', async () => {
    await testPersistentStorage(
      persistentStorage,
      'delete',
      'delete_me_key',
      'delete_me_config',
    );

    expect(persistentStorage.delete_mock).toHaveBeenCalledWith(
      'delete_me_key',
      'delete_me_config',
    );
  });

  it('does not throw when load returns garbage values', async () => {
    persistentStorage.load_mock.mockReturnValue({ a: ['b'] });

    await testPersistentStorage(persistentStorage, 'load');

    expect(persistentStorage.load_mock).toHaveBeenCalled();
  });

  it('does not throw when load returns null', async () => {
    persistentStorage.load_mock.mockReturnValue(null);

    await testPersistentStorage(persistentStorage, 'load');

    expect(persistentStorage.load_mock).toHaveBeenCalled();
  });
});
