import { DataStore } from '../../build/index.js';

export class MockDataStore implements DataStore {
  static verbose = false;
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
    MockDataStore.log('initialize');
    return Promise.resolve();
  }

  async shutdown() {
    MockDataStore.log('shutdown');
    return Promise.resolve();
  }

  async get(key: string) {
    MockDataStore.log('get', key);
    return Promise.resolve(this.data[key]);
  }

  async set(key: string, value: string, time?: number) {
    MockDataStore.log('set', key, value.slice(0, 10), time);
    this.data[key] = { result: value, time: time ?? 0 };
    return Promise.resolve();
  }

  async supportPollingUpdatesFor(key: string) {
    MockDataStore.log('supportPollingUpdatesFor', key);
    return Promise.resolve(this.supportPolling[key] === true);
  }

  private static log(message: string, ...args: any[]) {
    if (MockDataStore.verbose) {
      console.log('MockDataStore: ', message, ...args);
    }
  }
}
