import { readFileSync } from 'fs';

import type { SdkState } from '../common';
import { StatsigWrapper } from './statsig-wrapper';

const SCRAPI_URL = 'http://scrapi:8000';

for (let i = 0; i < 10; i++) {
  const res = await fetch(`${SCRAPI_URL}/ready`).catch(() => null);
  if (res?.status === 200) {
    break;
  }

  console.log('Waiting for scrapi to be ready');
  await new Promise((r) => setTimeout(r, 1000));
}

await StatsigWrapper.initialize();

function readSdkState(): SdkState {
  const state = readFileSync('/shared-volume/state.json', 'utf8');
  return JSON.parse(state).sdk;
}

function profile(name: string, fn: () => void) {
  const start = performance.now();
  fn();
  const end = performance.now();
  const duration = (end - start).toFixed(2);
  console.log(`${name} took ${duration}ms`);
}

function update() {
  console.log('--------------------------------------- [ Update ]');

  const start = performance.now();
  const state = readSdkState();

  console.log('Users: ', Object.keys(state.users).length);

  for (const userData of state.users) {
    StatsigWrapper.setUser(userData);

    const numGates = state.gate.names.length;
    profile(
      `checkGate(${numGates}) qps(${state.gate.qps}) user(${userData.userID})`,
      () => {
        for (const gateName of state.gate.names) {
          for (let i = 0; i < state.gate.qps; i++) {
            StatsigWrapper.checkGate(gateName);
          }
        }
      },
    );

    const numEvents = state.logEvent.events.length;
    profile(
      `logEvent(${numEvents}) qps(${state.logEvent.qps}) user(${userData.userID})`,
      () => {
        for (const event of state.logEvent.events) {
          for (let i = 0; i < state.logEvent.qps; i++) {
            StatsigWrapper.logEvent(event.eventName);
          }
        }
      },
    );
  }

  const end = performance.now();
  const duration = (end - start).toFixed(2);
  console.log(`Overall took ${duration}ms`);
}

setInterval(update, 1000);
