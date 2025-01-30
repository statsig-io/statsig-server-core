import {
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildNode(options: BuilderOptions) {
  Log.title(`Building statsig-node in Docker`);

  const { docker } = getArchInfo(options.arch);
  const tag = getDockerImageTag(options.distro, options.arch);
  const nodeDir = getRootedPath('statsig-node');

  const nodeCommand = [
    'pnpm install &&',
    'pnpm exec napi build',
    '--platform',
    '--js index.js',
    '--dts index.d.ts',
    options.release ? '--release --strip' : '',
    `--output-dir ${options.outDir ?? './build'}`,
  ].join(' ');

  const dockerCommand = [
    'docker run --rm',
    `--platform ${docker}`,
    `-v "${BASE_DIR}":/app`,
    `-v "/tmp:/tmp"`,
    `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
    `-v "/tmp/statsig-server-core/npm-cache:/root/.npm"`,
    tag,
    `"cd /app/statsig-node && ${nodeCommand}"`, // && while true; do sleep 1000; done
  ].join(' ');

  const command = isLinux(options.distro) ? dockerCommand : nodeCommand;

  Log.stepBegin(`Executing build command`);
  Log.stepProgress(command);

  execSync(command, { cwd: nodeDir, stdio: 'inherit' });

  Log.stepEnd(`Built statsig-node`);
}
