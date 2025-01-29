import { getDockerImageTag, getPlatformInfo } from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildPython(options: BuilderOptions) {
  const { docker } = getPlatformInfo(options.platform);
  const tag = getDockerImageTag(options.distro, options.platform);
  const pyDir = getRootedPath('statsig-pyo3');

  const maturinCommand = [
    'maturin build',
    options.release ? '--release' : '',
    options.outDir ? `--out ${options.outDir}` : '',
  ].join(' ');

  const dockerCommand = [
    'docker run --rm -it',
    `--platform ${docker}`,
    `-v "${BASE_DIR}":/app`,
    tag,
    `"cd /app/statsig-pyo3 && ${maturinCommand}"`,
  ].join(' ');

  Log.stepBegin(`Building Pyo3 Package ${tag}`);
  Log.stepProgress(dockerCommand);

  execSync(dockerCommand, { cwd: pyDir, stdio: 'inherit' });

  Log.stepEnd(`Built Pyo3 Package ${tag}`);
}
