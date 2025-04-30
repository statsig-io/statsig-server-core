import {
  OPERATING_SYSTEMS,
  OS,
  buildDockerImage,
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'node:child_process';
import fs from 'node:fs';

import { CommandBase } from './command_base.js';

export class VerifyPackage extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Verifies a the package is valid using a docker container',
      options: [
        {
          flags: '--tag <string>',
          description: 'The Dockerfile tag to use. eg amazonlinux2023',
          required: true,
        },
      ],
      args: [
        {
          name: '<package>',
          description: 'The package to verify',
          required: true,
        },
      ],
    });
  }

  override async run(packageName: string, options: { tag: string }) {
    Log.title(`Verifying Package`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`Package: ${packageName}`);
    Log.stepEnd(`Tag: ${options.tag}`);

    const dockerfile = getRootedPath(
      `cli/src/docker/verification-containers/Dockerfile.${packageName}_${options.tag}`,
    );

    if (!fs.existsSync(dockerfile)) {
      throw new Error(`Dockerfile ${dockerfile} does not exist`);
    }

    const sdkKey = process.env.STATSIG_SERVER_SDK_KEY;
    if (!sdkKey) {
      throw new Error('STATSIG_SERVER_SDK_KEY is not set');
    }

    const command = [
      'docker build',
      `-t server-core-verify-${packageName}-${options.tag}`,
      `-f ${dockerfile}`,
      `--build-arg STATSIG_SERVER_SDK_KEY=${sdkKey}`,
      '.',
    ].join(' ');

    Log.stepBegin('Building Docker Image');
    Log.stepEnd(command);

    execSync(command, {
      cwd: BASE_DIR,
      stdio: 'inherit',
    });

    Log.title('Package Verified');
  }
}
