import { StatsigNapiInternal, StatsigOptions } from './statsig-generated';

export * from './statsig-generated';

export class Statsig extends StatsigNapiInternal {
  constructor(sdkKey: string, options?: StatsigOptions) {
    super(sdkKey, options);
  }
}
