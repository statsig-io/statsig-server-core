import nodeFetch from 'node-fetch';

import { StatsigNapiInternal, StatsigOptions } from './statsig-generated';

export * from './statsig-generated';

const FETCH_FUNC = async (
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

export class Statsig extends StatsigNapiInternal {
  constructor(sdkKey: string, options?: StatsigOptions) {
    super(FETCH_FUNC, sdkKey, options);
  }
}
