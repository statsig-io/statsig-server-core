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
          flags: '--image-tag <string>',
          description: 'The docker image to use. eg amazonlinux2023',
          required: true,
        },
        {
          flags: '--package <string>',
          description: 'The package to verify',
          required: true,
        },
      ],
    });
  }

  override async run(opts: { package: string; imageTag: string }) {
    Log.title(`Verifying Package`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`Package: ${opts.package}`);
    Log.stepEnd(`Image Tag: ${opts.imageTag}`);

    const dockerfile = getRootedPath(
      `cli/src/docker/verification-containers/Dockerfile.${opts.package}_${opts.imageTag}`,
    );

    if (!fs.existsSync(dockerfile)) {
      Log.conclusion(`Dockerfile does not exist`, 'failure');
      Log.stepEnd(`${dockerfile}`);
      process.exit(1);
    }

    const sdkKey = process.env.STATSIG_SERVER_SDK_KEY;
    if (!sdkKey) {
      throw new Error('STATSIG_SERVER_SDK_KEY is not set');
    }

    const tag = `server-core-verify-${opts.package}-${opts.imageTag}`;

    const command = ['docker build', `-t ${tag}`, `-f ${dockerfile}`, '.'].join(
      ' ',
    );

    Log.stepBegin('Building Docker Image');
    Log.stepEnd(command);

    execSync(command, {
      cwd: BASE_DIR,
      stdio: 'inherit',
    });

    execSync(
      `docker run --rm --name ${tag} -e "STATSIG_SERVER_SDK_KEY=${sdkKey}" ${tag}`,
      {
        cwd: BASE_DIR,
        stdio: 'inherit',
      },
    );

    Log.title('Package Verified');
  }
}
