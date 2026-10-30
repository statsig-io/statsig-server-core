/**
 * Experimental APIs. These may change or be removed without notice.
 * Import from `@statsig/statsig-node-core/experimental` to opt in.
 */
import { experimental } from './statsig-generated';

export const ForwardProxyUserEnrichmentProvider =
  experimental.ForwardProxyUserEnrichmentProvider;
export type ForwardProxyUserEnrichmentProvider =
  experimental.ForwardProxyUserEnrichmentProvider;

export const StatsigFPUserBuilder = experimental.StatsigFPUserBuilder;
export type StatsigFPUserBuilder = experimental.StatsigFPUserBuilder;

export type ForwardProxyEnrichmentOptions =
  experimental.ForwardProxyEnrichmentOptions;
