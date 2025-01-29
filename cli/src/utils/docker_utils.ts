import { execSync } from 'node:child_process';

import { BASE_DIR, getRootedPath } from './file_utils.js';
import { Log } from './teminal_utils.js';

export type PlatformInfo = {
  docker: 'linux/amd64' | 'linux/arm64';
  name: 'arm64' | 'amd64';
  aliases: ['amd64', 'x86_64', 'x64'] | ['arm64', 'aarch64'];
};

export type Distro = 'debian' | 'amazonlinux2023' | 'amazonlinux2' | 'macos';
export type Platform = 'x64' | 'arm64' | 'amd64' | 'x86_64' | 'aarch64';

export function buildDockerImage(distro: Distro, platform: Platform = 'arm64') {
  const { docker } = getPlatformInfo(platform);
  const tag = getDockerImageTag(distro, platform);

  const command = [
    'docker build .',
    `--platform ${docker}`,
    `-t ${tag}`,
    `-f ${getRootedPath(`cli/src/docker/Dockerfile.${distro}`)}`,
    `--secret id=gh_token_id,env=GH_TOKEN`,
  ].join(' ');

  Log.stepBegin(`Building Server Core Docker Image`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  Log.stepEnd(`Built Server Core Docker Image`);
}

export function getDockerImageTag(distro: Distro, platform: Platform): string {
  const { name } = getPlatformInfo(platform);
  return `statsig/server-core-${distro}-${name}`;
}

export function getPlatformInfo(platform: Platform): PlatformInfo {
  if (platform === 'amd64' || platform === 'x86_64' || platform === 'x64') {
    return {
      docker: 'linux/amd64',
      name: 'amd64',
      aliases: ['amd64', 'x86_64', 'x64'],
    };
  }

  if (platform === 'arm64' || platform === 'aarch64') {
    return {
      docker: 'linux/arm64',
      name: 'arm64',
      aliases: ['arm64', 'aarch64'],
    };
  }

  throw new Error(`Unsupported platform: ${platform}`);
}
