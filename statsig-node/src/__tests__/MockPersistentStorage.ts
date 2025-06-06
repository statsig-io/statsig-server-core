import { PersistentStorage, StickyValues } from '../../build/index.js';

export class MockPersistentStorage implements PersistentStorage {
  readonly load_mock = jest.fn();
  readonly save_mock = jest.fn();
  readonly delete_mock = jest.fn();

  readonly load = (key: string): Record<string, StickyValues> | null => {
    // All these functions are specifically written this way to ensure
    // PersistentStorage typing does not break
    return this.load_mock(key);
  };

  readonly save = (
    key: string,
    config_name: string,
    data: StickyValues,
  ): void => {
    this.save_mock(key, config_name, data);
  };

  readonly delete = (key: string, config_name: string): void => {
    this.delete_mock(key, config_name);
  };
}
