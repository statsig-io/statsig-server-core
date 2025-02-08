import { DataStore } from '../../build/index.js';

export class MockDataStore implements DataStore {
  static verbose = false;
  data: Record<string, { result: string; time: number }> = {};
  supportPolling: Record<string, boolean> = {};

  readonly initialize = () => {
    this.log('initialize');
    return Promise.resolve();
  };

  readonly shutdown = () => {
    this.log('shutdown');
    return Promise.resolve();
  };

  readonly get = (key: string) => {
    this.log('get', key);
    return Promise.resolve(this.data[key]);
  };

  readonly set = (key: string, value: string, time?: number) => {
    this.log('set', key, value.slice(0, 10), time);
    this.data[key] = { result: value, time: time ?? 0 };
    return Promise.resolve();
  };

  readonly supportPollingUpdatesFor = (key: string) => {
    this.log('supportPollingUpdatesFor', key);
    return Promise.resolve(this.supportPolling[key] === true);
  };

  private log(message: string, ...args: any[]) {
    if (MockDataStore.verbose) {
      console.log('MockDataStore: ', message, ...args);
    }
  }
}
