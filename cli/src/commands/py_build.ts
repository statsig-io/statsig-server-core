import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import { Command } from 'commander';

type Options = {
  rebuildOpenssl?: boolean;
  target?: string;
  release?: boolean;
  out?: string;
  skipDockerBuild: boolean;
};

export class PyBuild extends Command {
  constructor() {
    super('py-build');

    this.description('Builds the statsig-pyo3 package');

    this.requiredOption(
      '--target, <string>',
      'Which target to build for, eg x86_64-apple-darwin',
    );
    this.option('--rebuild-openssl', 'Include vendored openssl with the build');
    this.option('--release', 'Build in release mode');
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
    Log.stepProgress(`Target: ${options.target ?? 'Not Specified'}`);
    Log.stepProgress(`For Release: ${options.release ?? false}`);
    Log.stepProgress(`Out Directory: ${options.out ?? 'Not Specified'}`);
    Log.stepEnd(`Rebuild OpenSSL: ${options.rebuildOpenssl ?? false}`);

    const image = getImage(options);
    const platform = getPlatform(options);

    if (!options.skipDockerBuild) {
      await buildDockerImage(image, platform);
    }

    await buildPyo3Package(image, platform, options);

    Log.conclusion('Successfully built statsig-pyo3');
  }
}

async function buildDockerImage(image: string, platform: string) {
  const pyDir = getRootedPath('statsig-pyo3');

  const command = [
    'docker build .',
    `--platform ${platform}`,
    `-t statsig/core-sdk-compiler:${image}`,
    `-f ${getRootedPath(`cli/src/docker/Dockerfile.${image}`)}`,
  ].join(' ');

  Log.stepBegin(`Building Docker Image ${image}`);
  Log.stepProgress(command);

  execSync(command, { cwd: pyDir, stdio: 'inherit' });

  Log.stepEnd(`Built Docker Image ${image}`);
}

async function buildPyo3Package(
  image: string,
  platform: string,
  options: Options,
) {
  const pyDir = getRootedPath('statsig-pyo3');

  const maturinCommand = [
    'maturin build',
    options.release ? '--release' : '',
    options.out ? `--out ${options.out}` : '',
  ].join(' ');

  const dockerCommand = [
    'docker run --rm -it',
    `--platform ${platform}`,
    `-v "${BASE_DIR}":/app`,
    `statsig/core-sdk-compiler:${image}`,
    `"cd /app/statsig-pyo3 && ${maturinCommand}"`,
  ].join(' ');

  Log.stepBegin(`Building Pyo3 Package ${image}`);
  Log.stepProgress(dockerCommand);

  execSync(dockerCommand, { cwd: pyDir, stdio: 'inherit' });

  Log.stepEnd(`Built Pyo3 Package ${image}`);
}

function getPlatform(options: Options) {
  switch (options.target) {
    case 'amazonlinux2023-arm64':
    case 'amazonlinux2-arm64':
      return 'linux/arm64';

    case 'amazonlinux2023-x86_64':
    case 'amazonlinux2-x86_64':
      return 'linux/amd64';

    default:
      throw new Error('Target is required');
  }
}

function getImage(options: Options) {
  if (options.target) {
    return options.target;
  }

  throw new Error('Target is required');
}
