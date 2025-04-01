import { Statsig, StatsigUser } from '@statsig/statsig-node-core';

(async function main() {
  const sdkKey = process.env.STATSIG_SERVER_SDK_KEY;
  if (!sdkKey) {
    throw new Error('STATSIG_SERVER_SDK_KEY is not set');
  }

  const statsig = new Statsig(sdkKey);
  await statsig.initialize();

  const user = StatsigUser.withUserID('a_user');
  user.custom = {
    os: process.platform,
    arch: process.arch,
    nodeVersion: process.version,
  };

  const gate = statsig.checkGate(user, 'test_public');
  const gcir = statsig.getClientInitializeResponse(user);

  console.log(
    '-------------------------------- Get Client Initialize Response --------------------------------',
  );
  console.log(JSON.stringify(JSON.parse(gcir), null, 2));
  console.log(
    '-------------------------------------------------------------------------------------------------',
  );

  console.log('Gate test_public: ', gate);

  if (!gate) {
    throw new Error('"test_public" gate is false but should be true');
  }

  const gcirJson = JSON.parse(gcir);
  if (Object.keys(gcirJson).length < 1) {
    throw new Error('GCIR is missing required fields');
  }

  console.log('All checks passed, shutting down...');
  await statsig.shutdown();
  console.log('Shutdown complete');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
