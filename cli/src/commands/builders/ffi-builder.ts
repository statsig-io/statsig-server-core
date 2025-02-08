import {
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildFfi(options: BuilderOptions) {
  Log.title(`Building statsig-ffi`);

  const { docker } = getArchInfo(options.arch);
  const tag = getDockerImageTag(options.os, options.arch);

  const cargoCommand = [
    'cargo build',
    '-p statsig_ffi',
    options.release ? '--release' : '',
  ].join(' ');

  const dockerCommand = [
    'docker run --rm',
    `--platform ${docker}`,
    `-v "${BASE_DIR}":/app`,
    `-v "/tmp:/tmp"`,
    `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
    tag,
    `"cd /app && ${cargoCommand}"`, // && while true; do sleep 1000; done
  ].join(' ');

  const command =
    isLinux(options.os) && options.docker ? dockerCommand : cargoCommand;

  Log.stepBegin(`Executing build command`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  Log.stepEnd(`Built statsig-node`);
}
