import {
  ARCHITECTURES,
  OPERATING_SYSTEMS,
  buildDockerImage,
} from '@/utils/docker_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builders/builder-options.js';
import { buildFfi } from './builders/ffi-builder.js';
import { buildJava } from './builders/java-builder.js';
import { buildNode } from './builders/node-builder.js';
import { buildPython } from './builders/python-builder.js';
import { CommandBase } from './command_base.js';
import { buildDotnet } from './builders/dotnet-builder.js';

const PACKAGES = ['python', 'node', 'java', 'ffi', 'dotnet'] as const;

const BUILDERS: Record<Package, (options: BuilderOptions) => void> = {
  python: buildPython,
  node: buildNode,
  java: buildJava,
  ffi: buildFfi,
  dotnet: buildDotnet
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
          choices: ARCHITECTURES,
        },
        {
          flags: '--os <string>',
          description: 'The operating system to build for',
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
        {
          flags: '-t, --target <string>',
          description:
            'The target to build for. (e.g. x86_64-unknown-linux-gnu)',
        },
        {
          flags: '-n, --no-docker',
          description: 'Prevent docker from being used',
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

    if (
      !options.docker &&
      (!options.arch || !options.os) &&
      !['node', 'python'].includes(packageName)
    ) {
      Log.stepEnd(
        'Must specify --arch and --os when --no-docker is set',
        'failure',
      );
      process.exit(1);
    }

    options.arch ??= 'arm64';
    options.os ??= 'debian';

    Log.stepBegin('Configuration');
    Log.stepProgress(`OS: ${options.os}`);
    Log.stepProgress(`Arch: ${options.arch}`);
    Log.stepProgress(`Target: ${options.target ?? 'Not Specified'}`);
    Log.stepProgress(`For Release: ${options.release}`);
    Log.stepProgress(`Out Directory: ${options.outDir ?? 'Not Specified'}`);
    Log.stepProgress(`Docker: ${options.docker}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    printToolingVersionInfo();

    if (!options.skipDockerBuild && options.docker) {
      buildDockerImage(options.os, options.arch);
    }

    BUILDERS[packageName](options);

    Log.conclusion(`Successfully built ${packageName}`);
  }
}

function printToolingVersionInfo() {
  Log.stepBegin('Tooling Versions');

  const rustc = execSync('rustc --version').toString();
  const cargo = execSync('cargo --version').toString();
  const rustfmt = execSync('rustfmt --version').toString();
  const clippy = execSync('clippy-driver --version').toString();

  const entries = Object.entries({ rustc, cargo, rustfmt, clippy });

  entries.forEach(([key, value], i) => {
    if (i === entries.length - 1) {
      Log.stepEnd(`${key}: ${value.trim()}`);
    } else {
      Log.stepProgress(`${key}: ${value.trim()}`);
    }
  });
}
