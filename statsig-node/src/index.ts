import {
  DataStore,
  DataStoreResponse,
  DynamicConfig,
  DynamicConfigEvaluationOptions,
  EvaluationDetails,
  Experiment,
  ExperimentEvaluationOptions,
  FeatureGate,
  FeatureGateEvaluationOptions,
  Layer,
  LayerEvaluationOptions,
  ObservabilityClient,
  OverrideAdapterConfig,
  OverrideAdapterType,
  SecondaryExposure,
  SpecAdapterConfig,
  StatsigOptions,
  __internal__testObservabilityClient,
  __internal__testDataStore
} from './binding';
import { Statsig } from './Statsig';

export {
  Statsig,
  Layer,
  Experiment,
  DynamicConfig,
  FeatureGate,
  StatsigOptions,
  FeatureGateEvaluationOptions,
  LayerEvaluationOptions,
  DynamicConfigEvaluationOptions,
  ExperimentEvaluationOptions,
  EvaluationDetails,
  SecondaryExposure,
  SpecAdapterConfig,
  OverrideAdapterConfig,
  OverrideAdapterType,
  DataStore,
  DataStoreResponse,
  ObservabilityClient,
  __internal__testObservabilityClient,
  __internal__testDataStore
};

export type StatsigUser = (
  | { userID: string }
  | { customIDs: Record<string, string> }
) & {
  userID?: string;
  customIDs?: Record<string, string>;
  email?: string;
  ip?: string;
  userAgent?: string;
  country?: string;
  locale?: string;
  appVersion?: string;
  custom?: Record<
    string,
    string | number | boolean | Array<string | number | boolean>
  >;
  privateAttributes?: Record<
    string,
    string | number | boolean | Array<string | number | boolean>
  >;
};