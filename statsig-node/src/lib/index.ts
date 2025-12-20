import HttpAgent, { HttpsAgent } from 'agentkeepalive';
import { HttpProxyAgent, HttpsProxyAgent } from 'hpagent';
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

// Create shared HTTP agents with keep-alive and proper timeout settings
// agentkeepalive provides freeSocketTimeout support for all Node versions
// and handles connection pooling more robustly than built-in agents
const httpAgent = new HttpAgent({
  // Defaults: keepAlive=true, freeSocketTimeout=4000ms, timeout=8000ms
  // Bump timeout to match SDK's default request timeout
  timeout: 30000,
});

const httpsAgent = new HttpsAgent({
  timeout: 30000,
});

// Agent options with keepAlive settings to prevent EPIPE errors
const agentOptions = {
  keepAlive: true,
  keepAliveMsecs: 1000,
  timeout: 30000,
  freeSocketTimeout: 15000,
  scheduling: 'fifo' as const,
};

function createProxyAgents(options?: StatsigOptions) {
  const proxy = options?.proxyConfig;
  if (proxy?.proxyHost && proxy?.proxyProtocol) {
    const protocol = proxy.proxyProtocol;
    const host = proxy.proxyHost;
    const port = proxy.proxyPort ? `:${proxy.proxyPort}` : '';
    const auth = proxy.proxyAuth ? `${proxy.proxyAuth}@` : '';
    const proxyUrl = `${protocol}://${auth}${host}${port}`;

    if (protocol === 'http' || protocol === 'https') {
      // hpagent supports all standard agent options including keepAlive/freeSocketTimeout
      return {
        http: new HttpProxyAgent({ proxy: proxyUrl, ...agentOptions }),
        https: new HttpsProxyAgent({ proxy: proxyUrl, ...agentOptions }),
      };
    }
  }
  return undefined;
}

function getAgent(
  url: string,
  proxyAgents?: { http: HttpProxyAgent; https: HttpsProxyAgent },
) {
  if (proxyAgents) {
    return url.startsWith('https') ? proxyAgents.https : proxyAgents.http;
  }
  return url.startsWith('https') ? httpsAgent : httpAgent;
}

function createFetchFunc(options?: StatsigOptions) {
  const proxyAgents = createProxyAgents(options);

  return async (
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: Uint8Array,
  ) => {
    try {
      const res = await nodeFetch(url, {
        method,
        headers: {
          ...headers,
          'Accept-Encoding': 'gzip, deflate, br',
        },
        body: body ? Buffer.from(body) : undefined,
        agent: getAgent(url, proxyAgents),
      });

      const data = await res.arrayBuffer();
      const resHeaders = Object.fromEntries(res.headers.entries()) as Record<
        string,
        string
      >;

      return {
        status: res.status,
        data: Array.from(new Uint8Array(data)),
        headers: resHeaders,
      };
    } catch (err: any) {
      return {
        status: 0,
        error: 'message' in err ? err.message : 'Unknown Node Fetch Error',
      };
    }
  };
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
    const fetchFunc = createFetchFunc(options);
    super(fetchFunc, sdkKey, options);

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

