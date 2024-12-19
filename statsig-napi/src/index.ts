import {
  SpecsAdapterTypeNapi as SpecsAdapterType,
  DynamicConfigNapi,
  ExperimentNapi,
  LayerNapi,
  SpecAdapterConfigNapi as SpecAdapterConfig,
} from './bindings';

import {
  IDataStore,
  getDataStoreKey,
} from './IDataStore'
import { Statsig, TypedGet } from './Statsig';
import StatsigUser, {StatsigUserArgs} from './StatsigUser';
import StatsigOptions, {StatsigOptionArgs, IObservabilityClient, LogLevel} from './StatsigOptions';
export type DynamicConfig = DynamicConfigNapi & {
  readonly value: Record<string, unknown>;
  readonly get: TypedGet;
};

export type Experiment = ExperimentNapi & {
  readonly value: Record<string, unknown>;
  readonly get: TypedGet;
};

export type Layer = LayerNapi & {
  readonly get: TypedGet;
};


export { SpecAdapterConfig, SpecsAdapterType, IDataStore, getDataStoreKey, Statsig, StatsigOptions, StatsigOptionArgs, StatsigUser, StatsigUserArgs, IObservabilityClient, LogLevel };

