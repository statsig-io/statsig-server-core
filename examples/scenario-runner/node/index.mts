import { readFileSync } from 'fs';

import type { SdkState } from '../common';
import { StatsigWrapper } from './statsig-wrapper';

const SCRAPI_URL = 'http://scrapi:8000';

for (let i = 0; i < 10; i++) {
  const res = await fetch(`${SCRAPI_URL}/v2/download_config_specs/xx`).catch(
    () => null,
  );
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

function update() {
  console.log('--------------------------------------- [ Update ]');

  const state = readSdkState();

  console.log('Users: ', Object.keys(state.users).length);
  console.log(
    `Gates: count(${state.gate.names.length}) qps(${state.gate.qps})`,
  );
  console.log(
    `Events: count(${state.logEvent.events.length}) qps(${state.logEvent.qps})`,
  );

  for (const userData of state.users) {
    StatsigWrapper.setUser(userData);

    for (const gateName of state.gate.names) {
      for (let i = 0; i < state.gate.qps; i++) {
        StatsigWrapper.checkGate(gateName);
      }
    }

    for (const event of state.logEvent.events) {
      for (let i = 0; i < state.logEvent.qps; i++) {
        StatsigWrapper.logEvent(event.eventName);
      }
    }
  }
}

setInterval(update, 1000);
