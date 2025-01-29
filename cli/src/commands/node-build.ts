import {
  Distro,
  Platform,
  buildDockerImage,
  getDockerImageTag,
  getPlatformInfo,
} from '@/utils/docker_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'node:child_process';

import { CommandBase } from './command_base.js';

type Options = {
  release: boolean;
  skipDockerBuild: boolean;
  platform: Platform;
  distro: Distro;
};

export class NodeBuild extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Builds the statsig-napi package');
    this.option('--release', 'Build in release mode', false);
    this.option(
      '-sdb, --skip-docker-build',
      'Skip building the docker image',
      false,
    );

    this.option(
      '-p, --platform <string>',
      'The platform to build for, e.g. x64 or arm64',
      'arm64',
    );

    this.option(
      '-d, --distro <string>',
      'The distro to build for. eg debian',
      'debian',
    );
  }

  async run(options: Options) {
    Log.title('Building statsig-node');

    if (!options.skipDockerBuild) {
      buildDockerImage(options.distro, options.platform);
    }

    buildNodePackageInDocker(options);

    Log.conclusion('Successfully built statsig-node');
  }
}

function buildNodePackageInDocker(options: Options) {
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
