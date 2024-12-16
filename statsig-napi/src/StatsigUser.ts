import {
  AutoReleasingStatsigUserRef,
  statsigUserCreate,
} from './bindings';

export type StatsigUserArgs = {
  userID: string,
  customIDs: Record<string, string>,
  email?: string | undefined | null,
  ip?: string | undefined | null,
  userAgent?: string | undefined | null,
  country?: string | undefined | null,
  locale?: string | undefined | null,
  appVersion?: string | undefined | null,
  custom?: Record<string, string> | undefined | null,
  privateAttributes?: Record<string, string> | undefined | null,
}

export default class StatsigUser {
  readonly __ref: AutoReleasingStatsigUserRef;

  constructor(
   args: StatsigUserArgs 
  ) {
    this.__ref = statsigUserCreate(
      args.userID,
      JSON.stringify(args.customIDs),
      args.email,
      args.ip,
      args.userAgent,
      args.country,
      args.locale,
      args.appVersion,
      args.custom ? JSON.stringify(args.custom) : null,
      args.privateAttributes ? JSON.stringify(args.privateAttributes) : null,
    );
  }
}