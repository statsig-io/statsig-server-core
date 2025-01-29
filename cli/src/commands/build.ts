import { buildDockerImage } from '@/utils/docker_utils.js';
import { Log } from '@/utils/teminal_utils.js';

import { BuilderOptions } from './builders/builder-options.js';
import { buildNode } from './builders/node-builder.js';
import { buildPython } from './builders/python-builder.js';
import { CommandBase } from './command_base.js';

const PACKAGES = ['python', 'node'] as const;

const BUILDERS: Record<Package, (options: BuilderOptions) => void> = {
  python: buildPython,
  node: buildNode,
};

type Package = (typeof PACKAGES)[number];

export class Build extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Builds the specified package');

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

    this.option('-o, --out-dir, <string>', 'Output directory');

    this.option(
      '-sdb, --skip-docker-build',
      'Skip building the docker image',
      false,
    );

    const arg = this.createArgument('<package>', 'The package to build');
    arg.choices(PACKAGES);
    this.addArgument(arg);
  }

  override async run(packageName: string, options: BuilderOptions) {
    Log.title(`Building ${packageName}`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`Distribution: ${options.distro}`);
    Log.stepProgress(`Platform: ${options.platform}`);
    Log.stepProgress(`For Release: ${options.release}`);
    Log.stepProgress(`Out Directory: ${options.outDir ?? 'Not Specified'}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    if (!options.skipDockerBuild) {
      buildDockerImage(options.distro, options.platform);
    }

    BUILDERS[packageName](options);

    Log.conclusion(`Successfully built ${packageName}`);
  }
}
