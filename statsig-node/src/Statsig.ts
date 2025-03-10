import { StatsigUser } from '.';
import {
  ClientInitResponseOptions,
  DynamicConfig,
  DynamicConfigEvaluationOptions,
  Experiment,
  ExperimentEvaluationOptions,
  FeatureGate,
  FeatureGateEvaluationOptions,
  Layer,
  LayerEvaluationOptions,
  OverrideAdapterType,
  Statsig as StatsigInternal,
  StatsigOptions,
  StatsigResult,
  StatsigUser as StatsigUserInternal,
} from './binding';

export class Statsig {
  private userFactory = new WeakMap<StatsigUser, StatsigUserInternal>();
  private statsigInternal: StatsigInternal;
  constructor(sdkKey: string, options: StatsigOptions) {
    this.statsigInternal = new StatsigInternal(sdkKey, options);
  }

  initialize(): Promise<StatsigResult> {
    return this.statsigInternal.initialize();
  }

  shutdown(timeoutMs?: number | undefined | null): Promise<StatsigResult> {
    return this.statsigInternal.shutdown(timeoutMs);
  }

  flushEvents(): Promise<StatsigResult> {
    return this.statsigInternal.flushEvents();
  }

  logEvent(
    user: StatsigUser,
    eventName: string,
    value?: string | number | null,
    metadata?: Record<string, string> | undefined | null,
  ): void {
    this.statsigInternal.logEvent(
      this.toInternalUser(user),
      eventName,
      value,
      metadata,
    );
  }

  checkGate(
    user: StatsigUser,
    gateName: string,
    options?: FeatureGateEvaluationOptions | undefined | null,
  ): boolean {
    return this.statsigInternal.checkGate(
      this.toInternalUser(user),
      gateName,
      options,
    );
  }

  getFeatureGate(
    user: StatsigUser,
    gateName: string,
    options?: FeatureGateEvaluationOptions | undefined | null,
  ): FeatureGate {
    return this.statsigInternal.getFeatureGate(
      this.toInternalUser(user),
      gateName,
      options,
    );
  }

  getFieldsNeededForGate(gateName: string): Array<string> {
    return this.statsigInternal.getFieldsNeededForGate(gateName);
  }

  getDynamicConfig(
    user: StatsigUser,
    configName: string,
    options?: DynamicConfigEvaluationOptions | undefined | null,
  ): DynamicConfig {
    return this.statsigInternal.getDynamicConfig(
      this.toInternalUser(user),
      configName,
      options,
    );
  }

  getFieldsNeededForDynamicConfig(configName: string): Array<string> {
    return this.statsigInternal.getFieldsNeededForDynamicConfig(configName);
  }

  getExperiment(
    user: StatsigUser,
    experimentName: string,
    options?: ExperimentEvaluationOptions | undefined | null,
  ): Experiment {
    return this.statsigInternal.getExperiment(
      this.toInternalUser(user),
      experimentName,
      options,
    );
  }

  getFieldsNeededForExperiment(experimentName: string): Array<string> {
    return this.statsigInternal.getFieldsNeededForExperiment(experimentName);
  }

  getLayer(
    user: StatsigUser,
    layerName: string,
    options?: LayerEvaluationOptions | undefined | null,
  ): Layer {
    return this.statsigInternal.getLayer(
      this.toInternalUser(user),
      layerName,
      options,
    );
  }

  getFieldsNeededForLayer(layerName: string): Array<string> {
    return this.statsigInternal.getFieldsNeededForLayer(layerName);
  }

  getClientInitializeResponse(
    user: StatsigUser,
    options?: ClientInitResponseOptions | undefined | null,
  ): string {
    return this.statsigInternal.getClientInitializeResponse(
      this.toInternalUser(user),
      options,
    );
  }

  manuallyLogFeatureGateExposure(user: StatsigUser, gateName: string): void {
    this.statsigInternal.manuallyLogFeatureGateExposure(
      this.toInternalUser(user),
      gateName,
    );
  }
  manuallyLogDynamicConfigExposure(
    user: StatsigUser,
    configName: string,
  ): void {
    this.statsigInternal.manuallyLogFeatureGateExposure(
      this.toInternalUser(user),
      configName,
    );
  }

  manuallyLogExperimentExposure(
    user: StatsigUser,
    experimentName: string,
  ): void {
    this.statsigInternal.manuallyLogExperimentExposure(
      this.toInternalUser(user),
      experimentName,
    );
  }

  manuallyLogLayerParamExposure(
    user: StatsigUser,
    layerName: string,
    paramName: string,
  ): void {
    this.statsigInternal.manuallyLogLayerParamExposure(
      this.toInternalUser(user),
      layerName,
      paramName,
    );
  }
  overrideGate(
    gateName: string,
    value: boolean,
    adapter?: OverrideAdapterType | undefined | null,
  ): void {
    this.statsigInternal.overrideGate(gateName, value, adapter);
  }
  overrideDynamicConfig(
    configName: string,
    value: Record<string, any>,
    adapter?: OverrideAdapterType | undefined | null,
  ): void {
    this.statsigInternal.overrideDynamicConfig(configName, value, adapter);
  }
  overrideExperiment(
    experimentName: string,
    value: Record<string, any>,
    adapter?: OverrideAdapterType | undefined | null,
  ): void {
    this.statsigInternal.overrideExperiment(experimentName, value, adapter);
  }
  overrideExperimentByGroupName(
    experimentName: string,
    groupName: string,
    adapter?: OverrideAdapterType | undefined | null,
  ): void {
    this.statsigInternal.overrideExperimentByGroupName(
      experimentName,
      groupName,
      adapter,
    );
  }
  overrideLayer(
    layerName: string,
    value: Record<string, any>,
    adapter?: OverrideAdapterType | undefined | null,
  ): void {
    this.overrideLayer(layerName, value, adapter);
  }

  private toInternalUser(user: StatsigUser): StatsigUserInternal {
    let userInternal = this.userFactory.get(user);
    if (userInternal == null) {
      userInternal = new StatsigUserInternal({ ...user });
      this.userFactory.set(user, userInternal);
    }

    return userInternal;
  }
}
