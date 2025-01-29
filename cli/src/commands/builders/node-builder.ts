import { getDockerImageTag, getPlatformInfo } from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildNode(options: BuilderOptions) {
  Log.title(`Building statsig-node in Docker`);

  const { docker } = getPlatformInfo(options.platform);
  const tag = getDockerImageTag(options.distro, options.platform);

  const command = [
    'docker run --rm -it',
    `--platform ${docker}`,
    `-v "${BASE_DIR}":/app`,
    `-v "/tmp:/tmp"`,
    `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
    `-v "/tmp/statsig-server-core/npm-cache:/root/.npm"`,
    tag,
    `"cd /app/statsig-node && pnpm build"`, // && while true; do sleep 1000; done
  ].join(' ');

  Log.stepBegin(`Executing build command`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  Log.stepEnd(`Built statsig-node`);
}
