import { __internal__testDataStore as testDataStore } from '../../build/index.js';
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

  it('should get values from the data store', async () => {
    dataStore.data[RULESET_KEY] = { result: 'test', time: 123 };

    const spy = jest.spyOn(dataStore, 'get');
    const [getResult] = await testDataStore(dataStore, RULESET_KEY, '');

    expect(spy).toHaveBeenCalledWith(RULESET_KEY);
    expect(getResult).toEqual({ result: 'test', time: 123 });
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

  it('should initialize and shutdown the data store', async () => {
    const initSpy = jest.spyOn(dataStore, 'initialize');
    const shutdownSpy = jest.spyOn(dataStore, 'shutdown');

    await testDataStore(dataStore, RULESET_KEY, '');

    expect(initSpy).toHaveBeenCalled();
    expect(shutdownSpy).toHaveBeenCalled();
  });
});
