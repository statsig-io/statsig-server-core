import {
  OPERATING_SYSTEMS,
  OS,
  buildDockerImage,
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'node:child_process';

import { CommandBase } from './command_base.js';

type ExecOptions = {
  os: OS;
  skipDockerBuild: boolean;
};

export class Exec extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Executes the specified command',
      options: [
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
      ],
      args: [
        {
          name: '<command...>',
          description: 'The command to execute',
          required: true,
        },
      ],
    });
  }

  override async run(commandArgs: string[], options: ExecOptions) {
    const command = commandArgs.join(' ');
    Log.title(`Executing ${command}`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`OS: ${options.os}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    const arch = 'aarch64';

    if (!options.skipDockerBuild) {
      buildDockerImage(options.os, arch);
    }

    const { docker } = getArchInfo(arch);
    const tag = getDockerImageTag(options.os, arch);

    const dockerCommand = [
      'docker run --rm',
      `--platform ${docker}`,
      `-v "${BASE_DIR}":/app`,
      `-v "/tmp:/tmp"`,
      `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
      `-e "test_api_key=${process.env.test_api_key}"`,
      tag,
      `"${command}"`,
    ].join(' ');

    const execCommand = isLinux(options.os) ? dockerCommand : command;

    Log.stepBegin(`Executing command`);
    Log.stepEnd(command);

    execSync(execCommand, { cwd: BASE_DIR, stdio: 'inherit' });
  }
}
