import { execSync } from 'node:child_process';

import { BASE_DIR, getRootedPath } from './file_utils.js';
import { Log } from './teminal_utils.js';

export type ArchInfo =
  | {
      docker: 'linux/arm64';
      name: 'aarch64';
      aliases: ['arm64', 'aarch64'];
    }
  | {
      docker: 'linux/amd64';
      name: 'x86_64';
      aliases: ['amd64', 'x86_64', 'x64'];
    }
  | {
      docker: 'ERROR_NOT_SUPPORTED';
      name: 'x86';
      aliases: ['i686', 'x86'];
    };

export const OPERATING_SYSTEMS = [
  'alpine',
  'amazonlinux2',
  'amazonlinux2023',
  'debian',
  'macos',
  'windows',
  'manylinux2014'
] as const;
export type OS = (typeof OPERATING_SYSTEMS)[number];

export const ARCHITECTURES = [
  'x64',
  'arm64',
  'amd64',
  'x86_64',
  'aarch64',
  'x86',
] as const;
export type Arch = (typeof ARCHITECTURES)[number];

export function isLinux(os: OS): boolean {
  return os !== 'windows' && os !== 'macos';
}

export function buildDockerImage(os: OS, arch: Arch = 'arm64') {
  const { docker } = getArchInfo(arch);
  const tag = getDockerImageTag(os, arch);

  const command = [
    'docker build .',
    `--platform ${docker}`,
    `-t ${tag}`,
    `-f ${getRootedPath(`cli/src/docker/Dockerfile.${os}`)}`,
    `--secret id=gh_token_id,env=GH_TOKEN`,
  ].join(' ');

  Log.stepBegin(`Building Server Core Docker Image`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  Log.stepEnd(`Built Server Core Docker Image`);
}

export function getDockerImageTag(os: OS, arch: Arch): string {
  const { name } = getArchInfo(arch);
  return `statsig/server-core-${os}-${name}`;
}

export function getArchInfo(arch: Arch): ArchInfo {
  if (arch === 'amd64' || arch === 'x86_64' || arch === 'x64') {
    return {
      docker: 'linux/amd64',
      name: 'x86_64',
      aliases: ['amd64', 'x86_64', 'x64'],
    };
  }

  if (arch === 'arm64' || arch === 'aarch64') {
    return {
      docker: 'linux/arm64',
      name: 'aarch64',
      aliases: ['arm64', 'aarch64'],
    };
  }

  if (arch === 'x86') {
    return {
      docker: 'ERROR_NOT_SUPPORTED' as any,
      name: 'x86',
      aliases: ['i686', 'x86'],
    };
  }

  throw new Error(`Unsupported architecture: ${arch}`);
}
