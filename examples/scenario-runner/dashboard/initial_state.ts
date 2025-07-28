import type { State } from '../common';

const EVAL_PROJ_SDK_KEY = process.env.EVAL_PROJ_SDK_KEY;

if (!EVAL_PROJ_SDK_KEY) {
  throw new Error('EVAL_PROJ_SDK_KEY is not set');
}

export const INITIAL_STATE: State = {
  chaosAgent: {
    active: true,
    lastChange: '',
    scenario: '',
    changeFrequencyMs: 60 * 60 * 1000,
  },
  scrapi: {
    dcs: {
      response: {
        v2Payload: '',
        v1Payload: '',
        status: 200,
        delayMs: 0,
      },
      syncing: {
        enabled: true,
        sdkKey: EVAL_PROJ_SDK_KEY,
        intervalMs: 10_000,
        updatedAt: new Date(),
      },
    },
    logEvent: {
      delayMs: 0,
      response: {
        status: 201,
        delayMs: 0,
        payload: '{ "success": true }',
      },
    },
  },
  sdk: {
    users: [
      { userID: 'a_user' },
      {
        userID: 'big_user',
        appVersion: '1.0.0',
        customIDs: { custom_id: '123' },
        privateAttributes: { private_attr: 'secret' },
        customAttributes: { custom_attr: 'custom_value' },
        ip: '127.0.0.1',
        country: 'US',
        email: 'test@test.com',
        userAgent:
          'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36',
        locale: 'en_US',
      },
    ],
    gate: {
      names: ['test_public'],
      qps: 1000,
    },
    logEvent: {
      events: [
        {
          eventName: 'my_custom_event',
        },
      ],
      qps: 1000,
    },
  },
};
