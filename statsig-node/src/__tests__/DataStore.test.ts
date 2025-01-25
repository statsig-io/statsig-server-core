import { DataStore, testDataStore } from '../../build/index.js';

class TestDataStore implements DataStore {
  data: Record<string, { result: string; time: number }> = {};
  supportPolling: Record<string, boolean> = {};

  constructor() {
    this.initialize = this.initialize.bind(this);
    this.shutdown = this.shutdown.bind(this);
    this.get = this.get.bind(this);
    this.set = this.set.bind(this);
    this.supportPollingUpdatesFor = this.supportPollingUpdatesFor.bind(this);
  }

  async initialize() {
    return Promise.resolve();
  }

  async shutdown() {
    return Promise.resolve();
  }

  async get(key: string) {
    return Promise.resolve(this.data[key]);
  }

  async set(key: string, value: string, time?: number) {
    this.data[key] = { result: value, time: time ?? 0 };
    return Promise.resolve();
  }

  async supportPollingUpdatesFor(key: string) {
    return Promise.resolve(this.supportPolling[key] === true);
  }
}

const RULESET_KEY = '/v2/download_config_specs';

describe('DataStore', () => {
  let dataStore: TestDataStore;

  beforeEach(async () => {
    dataStore = new TestDataStore();
  });

  afterAll(async () => {});

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
