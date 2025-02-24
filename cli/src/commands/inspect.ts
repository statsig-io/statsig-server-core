import {
  ARCHITECTURES,
  Arch,
  OPERATING_SYSTEMS,
  OS,
  buildDockerImage,
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR, listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'node:child_process';

import { CommandBase } from './command_base.js';

type InspectOptions = {
  dir: string;
  skipDockerBuild: boolean;
  docker: boolean;
  release: boolean;
  arch: Arch;
  os: OS;
  outDir: string;
};

export class Inspect extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Inspects built artifacts',
      options: [
        {
          flags: '-d, --dir <string>',
          description: 'The directory to inspect',
          required: true,
        },
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
          flags: '-sdb, --skip-docker-build',
          description: 'Skip building the docker image',
          defaultValue: false,
        },
        {
          flags: '-n, --no-docker',
          description: 'Prevent docker from being used',
        },
      ],
    });
  }

  override async run(options: InspectOptions) {
    Log.title(`Inspecting artifacts`);

    Log.stepBegin('Configuration');
    Log.stepEnd(`Dir: ${options.dir}`);

    Log.stepBegin('Finding artifacts');

    const binaries = [
      ...listFiles(options.dir, '**/*.a'),
      ...listFiles(options.dir, '**/*.dylib'),
      ...listFiles(options.dir, '**/*.so'),
      ...listFiles(options.dir, '**/*.dll'),
      ...listFiles(options.dir, '**/*.lib'),
    ];

    binaries.forEach((binary) => {
      Log.stepProgress(`Found: ${binary}`);
    });

    Log.stepEnd('Finished finding artifacts');

    if (!options.skipDockerBuild) {
      buildDockerImage(options.os, options.arch);
    }

    const { docker } = getArchInfo(options.arch);
    const tag = getDockerImageTag(options.os, options.arch);
    binaries.forEach((artifact) => {
      Log.stepBegin(`Inspecting: ${artifact}`);

      const command = `file ${artifact}`;

      const dockerCommand = [
        'docker run --rm',
        `--platform ${docker}`,
        `-v "${BASE_DIR}":/app`,
        `-v "/tmp:/tmp"`,
        `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
        tag,
        `"${command}"`,
      ].join(' ');

      const execCommand = isLinux(options.os) ? dockerCommand : command;

      Log.stepBegin(`Executing command`);
      Log.stepProgress(command);

      try {
        const output = execSync(execCommand, {
          cwd: BASE_DIR,
          stdio: 'pipe',
        });
        Log.stepEnd(output.toString());
      } catch (e) {
        Log.stepEnd(`Failed to inspect: ${artifact}`, 'failure');
        console.error(e);
      }
    });

    Log.conclusion(`Successfully inspected artifacts`);
  }
}
