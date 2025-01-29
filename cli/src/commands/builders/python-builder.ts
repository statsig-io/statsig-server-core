import { getDockerImageTag, getPlatformInfo } from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildPython(options: BuilderOptions) {
  const { docker } = getPlatformInfo(options.platform);
  const tag = getDockerImageTag(options.distro, options.platform);
  const pyDir = getRootedPath('statsig-pyo3');

  const target = getTarget(options);

  const maturinCommand = [
    'maturin build',
    options.release ? '--release' : '',
    options.outDir ? `--out ${options.outDir}` : '',
    target ? `--target ${target}` : '',
  ].join(' ');

  const dockerCommand = [
    'docker run --rm',
    `--platform ${docker}`,
    `-v "${BASE_DIR}":/app`,
    tag,
    `"cd /app/statsig-pyo3 && ${maturinCommand}"`,
  ].join(' ');

  const command = options.distro !== 'macos' ? dockerCommand : maturinCommand;

  Log.stepBegin(`Building Pyo3 Package ${tag}`);
  Log.stepProgress(command);

  execSync(command, { cwd: pyDir, stdio: 'inherit' });

  Log.stepEnd(`Built Pyo3 Package ${tag}`);
}

function getTarget(options: BuilderOptions) {
  if (options.distro !== 'macos') {
    return '';
  }

  const { name } = getPlatformInfo(options.platform);
  if (name === 'arm64') {
    return 'aarch64-apple-darwin';
  } else if (name === 'amd64') {
    return 'x86_64-apple-darwin';
  }

  throw new Error(`Unsupported platform: ${options.platform}`);
}
