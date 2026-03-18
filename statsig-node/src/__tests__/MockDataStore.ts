import { DataStore, DataStoreBytesResponse } from '../../build/index.js';

export class MockDataStore implements DataStore {
  static verbose = false;
  data: Record<string, { result: string; time: number }> = {};
  byteData: Record<string, DataStoreBytesResponse> = {};
  supportPolling: Record<string, boolean> = {};
  supportsBytes = true;

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

  readonly getBytes = (key: string) => {
    this.log('getBytes', key);
    return Promise.resolve(this.byteData[key]);
  };

  readonly set = (key: string, value: string, time?: number) => {
    this.log('set', key, value.slice(0, 10), time);
    this.data[key] = { result: value, time: time ?? 0 };
    return Promise.resolve();
  };

  readonly setBytes = (
    key: string,
    value: ArrayBuffer | Uint8Array,
    time?: number,
  ) => {
    const bytes = value instanceof Uint8Array ? value : new Uint8Array(value);
    this.log('setBytes', key, new TextDecoder().decode(bytes.slice(0, 10)), time);
    this.byteData[key] = { result: bytes, time: time ?? 0 };
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
