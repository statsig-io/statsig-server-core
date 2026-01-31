import { HttpsProxyAgent } from 'https-proxy-agent';
import nodeFetch from 'node-fetch';

import {
  startStatsigConsoleCapture,
  stopStatsigConsoleCapture,
} from './console_capture';
import { ErrorBoundary } from './error_boundary';
import {
  DynamicConfigEvaluationOptions,
  ExperimentEvaluationOptions,
  FeatureGateEvaluationOptions,
  LayerEvaluationOptions,
  ParameterStore,
  SdkEvent,
  StatsigNapiInternal,
  StatsigOptions,
  StatsigResult,
  StatsigUser,
} from './statsig-generated';
import { DynamicConfig, Experiment, FeatureGate, Layer } from './statsig_types';

export * from './statsig-generated';
export * from './statsig_types';

const inspectSym = Symbol.for('nodejs.util.inspect.custom');

// @ts-expect-error - prototype assignment
StatsigUser.prototype[inspectSym] = function () {
  return this.toJSON();
};
// @ts-expect-error - prototype assignment
ParameterStore.prototype[inspectSym] = function () {
  return this.toJSON();
};

function createProxyAgent(options?: StatsigOptions) {
  const proxy = options?.proxyConfig;
  if (proxy?.proxyHost && proxy?.proxyProtocol) {
    const protocol = proxy.proxyProtocol;
    const host = proxy.proxyHost;
    const port = proxy.proxyPort ? `:${proxy.proxyPort}` : '';
    const auth = proxy.proxyAuth ? `${proxy.proxyAuth}@` : '';
    const proxyUrl = `${protocol}://${auth}${host}${port}`;

    if (protocol === 'http' || protocol === 'https') {
      return new HttpsProxyAgent(proxyUrl);
    }
  }
  return undefined; // node-fetch agent parameter takes in undefined type instead of null
}
export class Statsig extends StatsigNapiInternal {
  private static _sharedInstance: Statsig | null = null;

  public static shared(): Statsig {
    if (!Statsig.hasShared()) {
      console.warn(
        '[Statsig] No shared instance has been created yet. Call newShared() before using it. Returning an invalid instance',
      );
      return _createErrorInstance();
    }
    return Statsig._sharedInstance!;
  }

  public static hasShared(): boolean {
    return Statsig._sharedInstance !== null;
  }

  public static newShared(sdkKey: string, options?: StatsigOptions): Statsig {
    if (Statsig.hasShared()) {
      console.warn(
        '[Statsig] Shared instance has been created, call removeSharedInstance() if you want to create another one. ' +
          'Returning an invalid instance',
      );
      return _createErrorInstance();
    }

    Statsig._sharedInstance = new Statsig(sdkKey, options);
    return Statsig._sharedInstance;
  }

  public static removeSharedInstance() {
    Statsig._sharedInstance = null;
  }

  constructor(sdkKey: string, options?: StatsigOptions) {
    super(sdkKey, options);

    ErrorBoundary.wrap(this);
    if (options?.consoleCaptureOptions?.enabled) {
      startStatsigConsoleCapture(sdkKey, options.consoleCaptureOptions);
    }
  }

  public stopConsoleCapture() {
    stopStatsigConsoleCapture();
  }

  public async shutdown(timeout_ms?: number): Promise<StatsigResult> {
    stopStatsigConsoleCapture();
    return super.shutdown(timeout_ms);
  }

  public subscribe(
    eventName: SdkEvent,
    callback: (event: any) => void,
  ): string {
    return this.__INTERNAL_subscribe(eventName, (raw) => {
      try {
        callback(JSON.parse(raw));
      } catch (error) {
        console.error(`[Statsig] Error parsing SDK Event: ${error}`);
      }
    });
  }

  public getFeatureGate(
    user: StatsigUser,
    gateName: string,
    options?: FeatureGateEvaluationOptions,
  ): FeatureGate {
    const raw = this.__INTERNAL_getFeatureGate(user, gateName, options);
    return new FeatureGate(gateName, raw);
  }

  public getDynamicConfig(
    user: StatsigUser,
    configName: string,
    options?: DynamicConfigEvaluationOptions,
  ): DynamicConfig {
    const raw = this.__INTERNAL_getDynamicConfig(user, configName, options);
    return new DynamicConfig(configName, raw);
  }

  public getExperiment(
    user: StatsigUser,
    experimentName: string,
    options?: ExperimentEvaluationOptions,
  ): Experiment {
    const raw = this.__INTERNAL_getExperiment(user, experimentName, options);
    return new Experiment(experimentName, raw);
  }

  public getExperimentByGroupName(
    experimentName: string,
    groupName: string,
  ): Experiment {
    const raw = this.__INTERNAL_getExperimentByGroupName(
      experimentName,
      groupName,
    );
    return new Experiment(experimentName, raw);
  }

  public getLayer(
    user: StatsigUser,
    layerName: string,
    options?: LayerEvaluationOptions,
  ): Layer {
    const raw = this.__INTERNAL_getLayer(user, layerName, options);
    return new Layer(
      (param) => this.__INTERNAL_logLayerParamExposure(raw, param),
      layerName,
      raw,
    );
  }
}

function _createErrorInstance(): Statsig {
  let dummyInstance = new Statsig('INVALID-KEY');
  dummyInstance.shutdown();
  return dummyInstance;
}
