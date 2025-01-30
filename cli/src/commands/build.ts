import {
  ARCHITECTURES,
  OPERATING_SYSTEMS,
  buildDockerImage,
} from '@/utils/docker_utils.js';
import { Log } from '@/utils/teminal_utils.js';

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

export class Build extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Builds the specified package',
      options: [
        {
          flags: '-a, --arch <string>',
          description: 'The architecture to build for',
          defaultValue: 'arm64',
          choices: ARCHITECTURES,
        },
        {
          flags: '--os <string>',
          description: 'The operating system to build for',
          defaultValue: 'debian',
          choices: OPERATING_SYSTEMS,
        },
        {
          flags: '-o, --out-dir <string>',
          description: 'Output directory',
        },
        {
          flags: '-r, --release',
          description: 'Build in release mode',
          defaultValue: false,
        },
        {
          flags: '-sdb, --skip-docker-build',
          description: 'Skip building the docker image',
          defaultValue: false,
        },
      ],
      args: [
        {
          name: '<package>',
          description: 'The package to build',
          choices: PACKAGES,
          required: true,
        },
      ],
    });
  }

  override async run(packageName: string, options: BuilderOptions) {
    Log.title(`Building ${packageName}`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`OS: ${options.os}`);
    Log.stepProgress(`Arch: ${options.arch}`);
    Log.stepProgress(`For Release: ${options.release}`);
    Log.stepProgress(`Out Directory: ${options.outDir ?? 'Not Specified'}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    if (!options.skipDockerBuild) {
      buildDockerImage(options.os, options.arch);
    }

    BUILDERS[packageName](options);

    Log.conclusion(`Successfully built ${packageName}`);
  }
}
