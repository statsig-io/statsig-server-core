type User = {
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
};

type Event = {
  eventName: string;
  value?: string | number;
  metadata?: Record<string, string>;
};

export type SdkState = {
  users: {
    [userID: string]: User;
  };
  gate: { names: string[]; qps: number };
  logEvent: {
    events: { [eventName: string]: Event };
    qps: number;
  };
  gcir: { qps: number };
};

export type ScrapiState = {
  dcs: {
    response: {
      v2Proto: {
        filepath: string;
        filesize: number;
      };
      v2: {
        filepath: string;
        filesize: number;
      };
      v1: {
        filepath: string;
        filesize: number;
      };
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
  updatedAt: Date;
};
