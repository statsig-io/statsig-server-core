import { LogLevel, Statsig } from "../dist/lib";
import { StatsigOptions } from "../dist/lib";
import { IDataStore } from "../src";
import { AdapterResponse, DataStoreKeyPath } from "../src/IDataStore";
import { StatsigUser } from "../dist/lib";
import test from 'ava';



class TestDataStore implements IDataStore {
  
  async get(key: String): Promise<AdapterResponse> {
    if (key.includes("download_config_specs")) {
      const response = JSON.stringify(require('../../statsig-lib/tests/data/eval_proj_dcs.json'));
      return {
        result: response,
        time: 123,
      }
    } else if (key.includes("get_id_lists")) {
      const response = JSON.stringify(require('../../statsig-lib/tests/data/get_id_lists'));
      return {
        result: response,
        time: 123,
      }
    } else {
      const response = JSON.stringify(require('../../statsig-lib/tests/data/company_id_list'));
      return {
        result: response,
        time: 123,
      }
    }

  }
  set(key: string, value: string, time?: number): Promise<void> {
    return Promise.resolve()
  }
  initialize(): Promise<void> {
    return Promise.resolve()
  }
  shutdown(): Promise<void> {
    return Promise.resolve()
  }
  supportsPollingUpdatesFor?(key: DataStoreKeyPath): boolean {
    return false
  }
}
test('Usage Example',async (t) => {
  const dataStore = new TestDataStore()
  const user = new StatsigUser({userID: "test-user", customIDs: {}});
  const statsigOptions = new StatsigOptions({
    outputLoggerLevel: LogLevel.Debug,
    environment: 'staging',
    dataStore: dataStore
  })
  const statsig = new Statsig("secret-key", statsigOptions);
  statsig.initialize();
  const gate = statsig.checkGate(user, "test_public");
  t.is(gate, true);
})