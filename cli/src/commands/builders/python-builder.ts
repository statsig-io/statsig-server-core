import {
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildPython(options: BuilderOptions) {
  const { docker } = getArchInfo(options.arch);
  const tag = getDockerImageTag(options.distro, options.arch);
  const pyDir = getRootedPath('statsig-pyo3');

  const target = getTarget(options);

  const maturinCommand = [
    'maturin build',
    '--sdist',
    options.release ? '--release --strip' : '',
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

  const command = isLinux(options.distro) ? dockerCommand : maturinCommand;

  Log.stepBegin(`Building Pyo3 Package ${tag}`);
  Log.stepProgress(command);

  execSync(command, { cwd: pyDir, stdio: 'inherit' });

  Log.stepEnd(`Built Pyo3 Package ${tag}`);
}

function getTarget(options: BuilderOptions) {
  const { name } = getArchInfo(options.arch);

  if (options.distro === 'macos') {
    switch (name) {
      case 'aarch64':
        return 'aarch64-apple-darwin';
      case 'x86_64':
        return 'x86_64-apple-darwin';
    }
  }

  if (options.distro === 'windows') {
    switch (name) {
      case 'aarch64':
        return 'aarch64-pc-windows-msvc';
      case 'x86_64':
        return 'x86_64-pc-windows-msvc';
      case 'x86':
        return 'i686-pc-windows-msvc';
    }
  }

  // linux figures it out by itself
  return '';
}
