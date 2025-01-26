import { execSync } from 'node:child_process';

import { BASE_DIR, getRootedPath } from './file_utils.js';
import { Log } from './teminal_utils.js';

export function buildDockerImage(flavor: 'debian') {
  const command = [
    'docker build .',
    `-t statsig/server-core-${flavor}`,
    `-f ${getRootedPath(`cli/src/docker/Dockerfile.${flavor}`)}`,
    `--secret id=gh_token_id,env=GH_TOKEN`,
  ].join(' ');

  Log.stepBegin(`Building Server Core Docker Image`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  Log.stepEnd(`Built Server Core Docker Image`);
}
