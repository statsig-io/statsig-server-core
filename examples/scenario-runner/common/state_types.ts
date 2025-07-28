export type SdkState = {
  users: {
    userID: string;
    appVersion?: string;
    customIDs?: Record<string, string>;
    privateAttributes?: Record<string, string>;
    customAttributes?: Record<string, string>;
    ip?: string;
    country?: string;
    email?: string;
    userAgent?: string;
    locale?: string;
  }[];
  gate: { names: string[]; qps: number };
  logEvent: { events: { eventName: string }[]; qps: number };
};

export type ScrapiState = {
  dcs: {
    response: {
      v2Payload: string;
      v1Payload: string;
      status: number;
      delayMs: number;
    };
    syncing: {
      enabled: boolean;
      sdkKey: string;
      intervalMs: number;
      updatedAt: Date;
    };
  };
  logEvent: {
    delayMs: number;
    response: {
      status: number;
      delayMs: number;
      payload: string;
    };
  };
};

export type State = {
  chaosAgent: {
    active: boolean;
    lastChange: string;
    scenario: string;
    changeFrequencyMs: number;
  };
  scrapi: ScrapiState;
  sdk: SdkState;
};
