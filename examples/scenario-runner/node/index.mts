import { execSync } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';

import type { SdkState } from '../common';
import { StatsigWrapper } from './statsig-wrapper';

const SCRAPI_URL = 'http://scrapi:8000';
const PROFILE_ARR: any[] = [];

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

function profile(
  name: string,
  userID: string,
  extra: string,
  qps: number,
  fn: () => void,
) {
  const durations: number[] = [];
  for (let i = 0; i < qps; i++) {
    const start = performance.now();
    fn();
    const end = performance.now();
    const duration = end - start;
    durations.push(duration);
  }

  const result: any = {
    name,
    userID,
    extra,
    qps,
  };

  if (qps > 0) {
    const sorted = durations.sort((a, b) => a - b);

    const median = sorted[Math.floor(sorted.length / 2)];
    const p99 = sorted[Math.floor(sorted.length * 0.99)];
    const min = sorted[0];
    const max = sorted[sorted.length - 1];
    result.median = median;
    result.p99 = p99;
    result.min = min;
    result.max = max;

    console.log(`${name} took ${p99}ms (p99), ${max}ms (max)`);
  }

  PROFILE_ARR.push(result);
}

function update() {
  console.log('--------------------------------------- [ Update ]');

  const start = performance.now();
  const state = readSdkState();

  PROFILE_ARR.length = 0;

  console.log('Users: ', Object.keys(state.users).length);

  for (const userData of state.users) {
    StatsigWrapper.setUser(userData);

    for (const gateName of state.gate.names) {
      profile(`check_gate`, userData.userID, gateName, state.gate.qps, () =>
        StatsigWrapper.checkGate(gateName),
      );
    }

    for (const event of state.logEvent.events) {
      profile(
        `log_event`,
        userData.userID,
        event.eventName,
        state.logEvent.qps,
        () => StatsigWrapper.logEvent(event.eventName),
      );
    }

    profile(`gcir`, userData.userID, '', state.gcir.qps, () =>
      StatsigWrapper.getClientInitResponse(),
    );
  }

  const end = performance.now();
  const duration = (end - start).toFixed(2);
  console.log(`Overall took ${duration}ms`);

  writeProfileData();
}

setInterval(update, 1000);

function writeProfileData() {
  const data = JSON.stringify(PROFILE_ARR, null, 2);
  const slug = `profile-node-${StatsigWrapper.isCore ? 'core' : 'legacy'}`;
  writeFileSync(`/shared-volume/${slug}-temp.json`, data);
  execSync(`mv /shared-volume/${slug}-temp.json /shared-volume/${slug}.json`);
}
