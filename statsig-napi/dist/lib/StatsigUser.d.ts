import { AutoReleasingStatsigUserRef } from './bindings';
export type StatsigUserArgs = {
    userID: string;
    customIDs: Record<string, string>;
    email?: string | undefined | null;
    ip?: string | undefined | null;
    userAgent?: string | undefined | null;
    country?: string | undefined | null;
    locale?: string | undefined | null;
    appVersion?: string | undefined | null;
    custom?: Record<string, string> | undefined | null;
    privateAttributes?: Record<string, string> | undefined | null;
};
export default class StatsigUser {
    readonly __ref: AutoReleasingStatsigUserRef;
    constructor(args: StatsigUserArgs);
}
