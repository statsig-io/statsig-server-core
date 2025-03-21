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
  const tag = getDockerImageTag(options.os, options.arch);
  const pyDir = getRootedPath('statsig-pyo3');

  const target = getTarget(options);

  // todo: fix stub gen for centos7
  const skipStubGen = options.os === 'centos7';

  const maturinCommand = [
    'maturin build',
    '--sdist',
    options.release ? '--release --strip' : '',
    options.outDir ? `--out ${options.outDir}` : '',
    target ? `--target ${target}` : '',
    skipStubGen ? '' : '&& cargo run --bin stub_gen',
  ].join(' ');

  const dockerCommand = [
    'docker run --rm',
    `--platform ${docker}`,
    `-v "${BASE_DIR}":/app`,
    `-v "/tmp/statsig-server-core/root-cargo-registry:/root/.cargo/registry"`,
    tag,
    `"cd /app/statsig-pyo3 && ${maturinCommand}"`,
  ].join(' ');

  const command =
    isLinux(options.os) && options.docker ? dockerCommand : maturinCommand;

  Log.stepBegin(`Building Pyo3 Package ${tag}`);
  Log.stepProgress(command);

  execSync(command, { cwd: pyDir, stdio: 'inherit' });

  Log.stepEnd(`Built Pyo3 Package ${tag}`);
}

function getTarget(options: BuilderOptions) {
  const { name } = getArchInfo(options.arch);

  if (options.os === 'macos') {
    switch (name) {
      case 'aarch64':
        return 'aarch64-apple-darwin';
      case 'x86_64':
        return 'x86_64-apple-darwin';
    }
  }

  if (options.os === 'windows') {
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
