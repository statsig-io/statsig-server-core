import {
  Distro,
  Platform,
  buildDockerImage,
  getDockerImageTag,
  getPlatformInfo,
} from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import { Command } from 'commander';

type Options = {
  release: boolean;
  platform: Platform;
  distro: Distro;
  out?: string;
  skipDockerBuild: boolean;
};

export class PyBuild extends Command {
  constructor() {
    super('py-build');

    this.description('Builds the statsig-pyo3 package');

    this.option(
      '-p, --platform <string>',
      'The platform to build for, e.g. x64 or arm64',
      'arm64',
    );

    this.option('-r, --release', 'Build in release mode', false);

    this.option(
      '-d, --distro <string>',
      'The distro to build for. eg debian',
      'debian',
    );

    this.option('--out, <string>', 'Output directory');

    this.option(
      '-sdb, --skip-docker-build',
      'Skip building the docker image',
      false,
    );

    this.action(this.run.bind(this));
  }

  async run(options: Options) {
    Log.title('Building statsig-pyo3');

    Log.stepBegin('Configuration');
    Log.stepProgress(`Distribution: ${options.distro}`);
    Log.stepProgress(`Platform: ${options.platform}`);
    Log.stepProgress(`For Release: ${options.release}`);
    Log.stepProgress(`Out Directory: ${options.out ?? 'Not Specified'}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    if (!options.skipDockerBuild) {
      buildDockerImage(options.distro, options.platform);
    }

    buildPyo3Package(options);

    Log.conclusion('Successfully built statsig-pyo3');
  }
}

function buildPyo3Package(options: Options) {
  const { docker } = getPlatformInfo(options.platform);
  const tag = getDockerImageTag(options.distro, options.platform);
  const pyDir = getRootedPath('statsig-pyo3');

  const maturinCommand = [
    'maturin build',
    options.release ? '--release' : '',
    options.out ? `--out ${options.out}` : '',
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
