import { DynamicConfig, Experiment, Layer } from '.';
import { AutoReleasingStatsigRef, ClientInitResponseOptions, FeatureGateNapi as FeatureGate, GetExperimentOptions, GetFeatureGateOptions, GetLayerOptions } from './bindings';
import StatsigOptions from './StatsigOptions';
import StatsigUser from './StatsigUser';
export declare class Statsig {
    readonly __ref: AutoReleasingStatsigRef;
    constructor(sdkKey: string, options?: StatsigOptions);
    initialize(): Promise<void>;
    shutdown(): Promise<void>;
    logEvent(user: StatsigUser, eventName: string, value?: string | number | undefined | null, metadata?: Record<string, string> | undefined | null): void;
    checkGate(user: StatsigUser, gateName: string, options?: GetFeatureGateOptions): boolean;
    getFeatureGate(user: StatsigUser, gateName: string, options?: GetFeatureGateOptions): FeatureGate;
    manuallyLogGateExposure(user: StatsigUser, gateName: string): void;
    getDynamicConfig(user: StatsigUser, dynamicConfigName: string, options?: GetExperimentOptions): DynamicConfig;
    manuallyLogDynamicConfigExposure(user: StatsigUser, configName: string): void;
    getExperiment(user: StatsigUser, experimentName: string, options?: GetExperimentOptions): Experiment;
    manuallyLogExperimentExposure(user: StatsigUser, gateName: string): void;
    getLayer(user: StatsigUser, layerName: string, options?: GetLayerOptions): Layer;
    manuallyLogLayerParameterExposure(user: StatsigUser, layerName: string, parameterName: string): void;
    getClientInitializeResponse(user: StatsigUser, options?: ClientInitResponseOptions): string;
}
export type TypedReturn<T = unknown> = T extends string ? string : T extends number ? number : T extends boolean ? boolean : T extends Array<unknown> ? Array<unknown> : T extends object ? object : unknown;
export type TypedGet = <T = unknown>(key: string, fallback?: T) => TypedReturn<T>;
