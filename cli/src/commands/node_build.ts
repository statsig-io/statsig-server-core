import { buildDockerImage } from '@/utils/docker_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { Command } from 'commander';
import { execSync } from 'node:child_process';

type Options = {
  release: boolean;
  skipDockerBuild: boolean;
};

export class NodeBuild extends Command {
  constructor() {
    super('node-build');

    this.description('Builds the statsig-napi package');
    this.option('--release', 'Build in release mode', false);
    this.option(
      '-sdb, --skip-docker-build',
      'Skip building the docker image',
      false,
    );

    this.action(this.run.bind(this));
  }

  async run(options: Options) {
    Log.title('Building statsig-node');

    if (!options.skipDockerBuild) {
      buildDockerImage('debian');
    }

    buildNodePackageInDocker();

    Log.conclusion('Successfully built statsig-node');
  }
}

function buildNodePackageInDocker() {
  Log.title(`Building statsig-node in Docker`);

  const command = [
    'docker run --rm -it',
    `-v "${BASE_DIR}":/app`,
    `-v "/tmp:/tmp"`,
    `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
    `-v "/tmp/statsig-server-core/npm-cache:/root/.npm"`,
    `statsig/server-core-debian`,
    `"cd /app/statsig-node && pnpm build"`, // && while true; do sleep 1000; done
  ].join(' ');

  Log.stepBegin(`Executing build command`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  Log.stepEnd(`Built statsig-node`);
}
