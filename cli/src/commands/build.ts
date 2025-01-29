import {
  ARCHITECTURES,
  DISTROS,
  buildDockerImage,
} from '@/utils/docker_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { Argument, Option } from 'commander';

import { BuilderOptions } from './builders/builder-options.js';
import { buildFfi } from './builders/ffi-builder.js';
import { buildNode } from './builders/node-builder.js';
import { buildPython } from './builders/python-builder.js';
import { CommandBase } from './command_base.js';

const PACKAGES = ['python', 'node', 'ffi'] as const;

const BUILDERS: Record<Package, (options: BuilderOptions) => void> = {
  python: buildPython,
  node: buildNode,
  ffi: buildFfi,
};

type Package = (typeof PACKAGES)[number];

function createOption(
  flags: string,
  description: string,
  defaultValue?: unknown,
  choices?: readonly string[],
) {
  const opt = new Option(flags, description);

  if (defaultValue) {
    opt.default(defaultValue);
  }

  if (choices) {
    opt.choices(choices);
  }

  return opt;
}

const OPTIONS = [
  createOption(
    '-a, --arch <string>',
    'The architecture to build for',
    'arm64',
    ARCHITECTURES,
  ),
  createOption(
    '-d, --distro <string>',
    'The distro to build for',
    'debian',
    DISTROS,
  ),
  createOption('-o, --out-dir <string>', 'Output directory'),
  createOption('-r, --release', 'Build in release mode', false),
  createOption(
    '-sdb, --skip-docker-build',
    'Skip building the docker image',
    false,
  ),
];

export class Build extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Builds the specified package');

    OPTIONS.forEach((opt) => this.addOption(opt));

    const arg = new Argument('<package>', 'The package to build');
    arg.choices(PACKAGES);
    this.addArgument(arg);
  }

  override async run(packageName: string, options: BuilderOptions) {
    Log.title(`Building ${packageName}`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`Distribution: ${options.distro}`);
    Log.stepProgress(`Arch: ${options.arch}`);
    Log.stepProgress(`For Release: ${options.release}`);
    Log.stepProgress(`Out Directory: ${options.outDir ?? 'Not Specified'}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    if (!options.skipDockerBuild) {
      buildDockerImage(options.distro, options.arch);
    }

    BUILDERS[packageName](options);

    Log.conclusion(`Successfully built ${packageName}`);
  }
}
