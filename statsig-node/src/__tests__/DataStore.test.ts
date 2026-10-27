import {
  __internal__testDataStore as testDataStore,
  __internal__testDataStoreFeature as testDataStoreFeature,
} from '../../build/index.js';
import { MockDataStore } from './MockDataStore';

const RULESET_KEY = '/v2/download_config_specs';

describe('DataStore', () => {
  let dataStore: MockDataStore;

  beforeEach(async () => {
    dataStore = new MockDataStore();
  });

  it('should check data store for polling support', async () => {
    dataStore.supportPolling[RULESET_KEY] = true;

    const spy = jest.spyOn(dataStore, 'supportPollingUpdatesFor');
    const [_, pollingResult] = await testDataStore(dataStore, RULESET_KEY, '');

    expect(spy).toHaveBeenCalledWith(RULESET_KEY);
    expect(pollingResult).toBe(true);
  });

  it('defaults legacy adapters to config specs only', async () => {
    const legacy = { ...dataStore, supportPollingUpdatesFor: undefined };

    expect(await testDataStoreFeature(legacy)).toEqual([true, false, null, false]);
  });

  it('passes the logical feature name to capability and polling callbacks', async () => {
    const supportsFeature = jest.fn(async (feature: string) =>
      feature === 'id_list_snapshot',
    );
    dataStore.supportPolling['id_list_snapshot'] = true;
    const polling = jest.spyOn(dataStore, 'supportPollingUpdatesFor');

    expect(
      await testDataStoreFeature({ ...dataStore, supportsFeature }),
    ).toEqual([false, true, true, false]);
    expect(supportsFeature).toHaveBeenCalledWith('config_specs');
    expect(supportsFeature).toHaveBeenCalledWith('id_list_snapshot');
    expect(supportsFeature).toHaveBeenCalledWith('id_list_snapshot_write');
    expect(polling).toHaveBeenCalledWith('id_list_snapshot');
    expect(polling).not.toHaveBeenCalledWith('id_list_snapshot_write');
  });

  it.each([
    ['missing', undefined, null],
    ['true', async () => true, true],
    ['false', async () => false, false],
    ['rejected promise', async () => Promise.reject(new Error('polling failed')), null],
    ['invalid return', async () => 'invalid', null],
  ])('handles %s ID-list polling callback', async (_, callback, expected) => {
    const supportsFeature = async (feature: string) =>
      feature === 'id_list_snapshot';
    const store = {
      ...dataStore,
      supportsFeature,
      supportPollingUpdatesFor: callback,
    } as unknown as MockDataStore;

    expect(await testDataStoreFeature(store)).toEqual([false, true, expected, false]);
  });

  it.each<[string, (() => Promise<unknown>) | undefined, boolean]>([
    ['absent', undefined, false],
    ['false', async () => false, false],
    ['true', async () => true, true],
    ['rejected promise', async () => Promise.reject(new Error('write unsupported')), false],
    ['invalid return', async () => 'invalid', false],
  ])('requires explicit valid %s write capability', async (_, callback, expected) => {
    const supportsFeature = async (feature: string) => {
      if (feature === 'id_list_snapshot_write') {
        return callback == null ? false : callback();
      }
      return feature === 'id_list_snapshot';
    };
    const store = { ...dataStore, supportsFeature } as unknown as MockDataStore;
    expect(await testDataStoreFeature(store)).toEqual([false, true, false, expected]);
  });

  it('should get values from the data store', async () => {
    dataStore.data[RULESET_KEY] = { result: 'test', time: 123 };

    const spy = jest.spyOn(dataStore, 'get');
    const [getResult] = await testDataStore(dataStore, RULESET_KEY, '');

    expect(spy).toHaveBeenCalledWith(RULESET_KEY);
    expect(getResult).toEqual({ result: 'test', time: 123 });
  });

  it('should get bytes from the data store', async () => {
    const storedBytes = new TextEncoder().encode('binary-test');
    dataStore.byteData[RULESET_KEY] = {
      result: storedBytes,
      time: 456,
    };

    const spy = jest.spyOn(dataStore, 'getBytes');
    const getBytesResult = await dataStore.getBytes(RULESET_KEY);

    expect(spy).toHaveBeenCalledWith(RULESET_KEY);
    expect(getBytesResult).toEqual({ result: storedBytes, time: 456 });
  });

  it('should set values in the data store', async () => {
    const spy = jest.spyOn(dataStore, 'set');
    await testDataStore(dataStore, RULESET_KEY, 'set_value');

    expect(spy).toHaveBeenCalledWith(RULESET_KEY, 'set_value', null);
    expect(dataStore.data[RULESET_KEY]).toEqual({
      result: 'set_value',
      time: 0,
    });
  });

  it('should set bytes in the data store', async () => {
    const spy = jest.spyOn(dataStore, 'setBytes');
    const bytes = new TextEncoder().encode('set-binary-value');

    await dataStore.setBytes(RULESET_KEY, bytes, 789);

    expect(spy).toHaveBeenCalledWith(RULESET_KEY, bytes, 789);
    expect(dataStore.byteData[RULESET_KEY]).toEqual({
      result: bytes,
      time: 789,
    });
  });

  it('should initialize and shutdown the data store', async () => {
    const initSpy = jest.spyOn(dataStore, 'initialize');
    const shutdownSpy = jest.spyOn(dataStore, 'shutdown');

    await testDataStore(dataStore, RULESET_KEY, '');

    expect(initSpy).toHaveBeenCalled();
    expect(shutdownSpy).toHaveBeenCalled();
  });
});
