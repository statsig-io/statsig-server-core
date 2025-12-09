import 'jest-extended';
import * as fs from 'node:fs';
import * as path from 'node:path';

import {
  Experiment,
  Layer,
  PersistentStorage,
  Statsig,
  StatsigUser,
  StickyValues,
} from '../../build/index.js';
import { MockScrapi } from './MockScrapi';

class TestPersistentStorage implements PersistentStorage {
  data: Record<string, Record<string, StickyValues>> = {};

  load = (key: string): Record<string, StickyValues> | null => {
    return this.data[key];
  };

  save = (key: string, config_name: string, data: StickyValues): void => {
    this.data[key] = { [config_name]: data };
  };

  delete = (key: string, config_name: string): void => {
    const found = this.data[key];
    if (found) {
      delete found[config_name];
    }
  };
}

describe('Persisted Storage Usage', () => {
  const user = new StatsigUser({
    userID: 'user-in-test',
  });

  let persistentStorage: TestPersistentStorage;
  let scrapi: MockScrapi;
  let statsig: Statsig;

  beforeAll(async () => {
    persistentStorage = new TestPersistentStorage();
    scrapi = await MockScrapi.create();

    scrapi.mock('/v2/download_config_specs', getDcs(), {
      status: 200,
      method: 'GET',
    });

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
    const logEventUrl = scrapi.getUrlForPath('/v1/log_event');

    statsig = new Statsig('secret-123', {
      specsUrl,
      logEventUrl,
      persistentStorage,
      specsSyncIntervalMs: 100,
      outputLogLevel: 'debug',
    });

    await statsig.initialize();
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  describe('persisted experiment with holdout', () => {
    let experiment: Experiment;

    beforeAll(() => {
      persistentStorage.data = {};
      experiment = statsig.getExperiment(
        user,
        'running_exp_in_layer_with_holdout',
        {
          userPersistedValues: persistentStorage.data,
        },
      );
    });

    it('should have the correct values', () => {
      expect(experiment.name).toBe('running_exp_in_layer_with_holdout');
      expect(experiment.groupName).toBe('Test');
      expect(experiment.ruleID).toBe('FC34CQnbBwlkcpMxdi8MT');
      expect(experiment.details.reason).toBe('Network:Recognized');
      expect(experiment.getValue('exp_val')).toBe('running_test');
      expect(experiment.getValue('layer_val')).toBe('layer_default');
    });

    it('has the correct secondary exposures', () => {
      expect(experiment.getSecondaryExposures()?.[0]).toEqual(
        expect.objectContaining({
          gate: 'global_holdout',
          gateValue: 'false',
          ruleID: '3QoA4ncNdVGBaMt3N1KYjz:0.50:1',
        }),
      );

      expect(experiment.getSecondaryExposures()?.[1]).toEqual(
        expect.objectContaining({
          gate: 'layer_holdout',
          gateValue: 'false',
          ruleID: '2bAVp6R3C85vCYrR6be36n:10.00:5',
        }),
      );
    });

    it('should write values to persistent storage', async () => {
      await waitFor(() => Object.keys(persistentStorage.data).length > 0);

      expect(persistentStorage.data['user-in-test:userID']).toContainKey(
        'running_exp_in_layer_with_holdout',
      );
    });
  });

  describe('persisted experiments', () => {
    let experiment: Experiment;

    describe('getting the experiment with no sticky values', () => {
      beforeAll(() => {
        persistentStorage.data = {};
        experiment = statsig.getExperiment(
          user,
          'experiment_with_many_params',
          {
            userPersistedValues: persistentStorage.data,
          },
        );
      });

      it('should have the correct values', () => {
        expect(experiment.name).toBe('experiment_with_many_params');
        expect(experiment.groupName).toBe('Test');
        expect(experiment.ruleID).toBe('7kGqFaUIGHjHJ5X7SOKJcM');
        expect(experiment.details.reason).toBe('Network:Recognized');
        expect(experiment.getValue('a_bool')).toBe(false);
        expect(experiment.getValue('a_string')).toBe('test_1');
      });

      it('should write values to persistent storage', async () => {
        await waitFor(() => Object.keys(persistentStorage.data).length > 0);

        expect(persistentStorage.data['user-in-test:userID']).toContainKey(
          'experiment_with_many_params',
        );
      });
    });

    describe('getting the experiment with sticky values', () => {
      beforeAll(() => {
        persistentStorage.data = {
          'user-in-test:userID': {
            experiment_with_many_params: {
              config_delegate: null,
              config_version: 21,
              explicit_parameters: ['a_string'],
              group_name: 'FakeGroupName',
              json_value: {
                a_bool: true,
                a_string: 'sticky_value',
              },
              rule_id: 'sticky_rule_id',
              secondary_exposures: [],
              time: 1765242332716,
              undelegated_secondary_exposures: null,
              value: true,
            },
          },
        } as any;

        experiment = statsig.getExperiment(
          user,
          'experiment_with_many_params',
          {
            userPersistedValues: persistentStorage.data['user-in-test:userID'],
          },
        );
      });

      it('should have the correct values', () => {
        expect(experiment.name).toBe('experiment_with_many_params');
        expect(experiment.groupName).toBe('FakeGroupName');
        expect(experiment.ruleID).toBe('sticky_rule_id');
        expect(experiment.details.reason).toBe('Persisted');
        expect(experiment.getValue('a_bool')).toBe(true);
        expect(experiment.getValue('a_string')).toBe('sticky_value');
      });

      it('should leave values in persistent storage unchanged', async () => {
        await waitFor(() => Object.keys(persistentStorage.data).length > 0);

        expect(persistentStorage.data['user-in-test:userID']).toContainKey(
          'experiment_with_many_params',
        );
      });
    });
  });

  describe('persisted layers', () => {
    let layer: Layer;

    describe('getting the layer with no sticky values', () => {
      beforeAll(() => {
        persistentStorage.data = {};
        layer = statsig.getLayer(user, 'layer_with_many_params', {
          userPersistedValues: persistentStorage.data,
        });
      });

      it('should have the correct values', () => {
        expect(layer.allocatedExperimentName).toBe(
          'experiment_with_many_params',
        );
        expect(layer.getGroupName()).toBe('Test');
        expect(layer.getRuleId()).toBe('7kGqFaUIGHjHJ5X7SOKJcM');
        expect(layer.getEvaluationDetails().reason).toBe('Network:Recognized');
        expect(layer.getValue('a_bool')).toBe(false);
        expect(layer.getValue('a_string')).toBe('test_1');
      });

      it('should write values to persistent storage', async () => {
        await waitFor(() => Object.keys(persistentStorage.data).length > 0);

        expect(persistentStorage.data['user-in-test:userID']).toContainKey(
          'layer_with_many_params',
        );
      });
    });

    describe('getting the layer with sticky values', () => {
      beforeAll(() => {
        persistentStorage.data = {
          'user-in-test:userID': {
            layer_with_many_params: {
              config_delegate: 'experiment_with_many_params',
              config_version: 19,
              explicit_parameters: ['a_string'],
              group_name: 'FakeGroupName',
              json_value: {
                a_bool: true,
                a_string: 'sticky_value',
              },
              rule_id: '7kGqFaUIGHjHJ5X7SOKJcM',
              secondary_exposures: [],
              time: 1765242332716,
              undelegated_secondary_exposures: [],
              value: true,
            },
          },
        } as any;

        layer = statsig.getLayer(user, 'layer_with_many_params', {
          userPersistedValues: persistentStorage.data['user-in-test:userID'],
        });
      });

      it('should have the correct values', () => {
        expect(layer.allocatedExperimentName).toBe(
          'experiment_with_many_params',
        );
        expect(layer.getGroupName()).toBe('FakeGroupName');
        expect(layer.getRuleId()).toBe('7kGqFaUIGHjHJ5X7SOKJcM');
        expect(layer.getEvaluationDetails().reason).toBe('Persisted');
        expect(layer.getValue('a_bool')).toBe(true);
        expect(layer.getValue('a_string')).toBe('sticky_value');
      });

      it('should leave values in persistent storage unchanged', async () => {
        await waitFor(() => Object.keys(persistentStorage.data).length > 0);

        expect(persistentStorage.data['user-in-test:userID']).toContainKey(
          'layer_with_many_params',
        );
      });
    });
  });
});

async function waitFor(action: () => boolean) {
  for (let i = 0; i < 10; i++) {
    if (action()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }

  throw new Error('Timed out waiting for action to succeed');
}

function getDcs() {
  return fs.readFileSync(
    path.join(__dirname, '../../../statsig-rust/tests/data/eval_proj_dcs.json'),
    'utf8',
  );
}
