import { HttpsProxyAgent } from 'https-proxy-agent';
import nodeFetch from 'node-fetch';

import { ErrorBoundary } from './error_boundary';
import { StatsigNapiInternal, StatsigOptions } from './statsig-generated';

export * from './statsig-generated';

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

function createFetchFunc(options?: StatsigOptions) {
  const proxyAgent = createProxyAgent(options);

  return async (
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: Uint8Array,
  ) => {
    try {
      const res = await nodeFetch(url, {
        method,
        headers,
        body: body ? Buffer.from(body) : undefined,
        agent: proxyAgent,
      });

      const data = await res.arrayBuffer();

      return {
        status: res.status,
        data: Array.from(new Uint8Array(data)),
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
  constructor(sdkKey: string, options?: StatsigOptions) {
    const fetchFunc = createFetchFunc(options);
    super(fetchFunc, sdkKey, options);

    ErrorBoundary.wrap(this);
  }
}
