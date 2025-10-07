import { ensureEmptyDir, listFiles } from '@/utils/file_utils.js';
import { getCurrentCommitHash } from '@/utils/git_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/terminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { execSync, spawnSync } from 'node:child_process';
import path from 'node:path';
import { simpleGit } from 'simple-git';

import { PublisherOptions } from './publisher-options.js';

const TEMP_PATH = '/tmp/server-core-go';

const ASSET_MAPPING = {
  'go-server-core-binaries-linux-gnu': {
    // AARCH64
    'aarch64-unknown-linux-gnu/release/libstatsig_ffi.so':
      'linux_gnu_aarch64.so',
    'aarch64-unknown-linux-gnu/release/libstatsig_ffi.so.sig':
      'linux_gnu_aarch64.so.sig',
    // X86_64
    'x86_64-unknown-linux-gnu/release/libstatsig_ffi.so': 'linux_gnu_x86_64.so',
    'x86_64-unknown-linux-gnu/release/libstatsig_ffi.so.sig':
      'linux_gnu_x86_64.so.sig',
  },
  'go-server-core-binaries-linux-musl': {
    // AARCH64
    'aarch64-unknown-linux-musl/release/libstatsig_ffi.so':
      'linux_musl_aarch64.so',
    'aarch64-unknown-linux-musl/release/libstatsig_ffi.so.sig':
      'linux_musl_aarch64.so.sig',
    // X86_64
    'x86_64-unknown-linux-musl/release/libstatsig_ffi.so':
      'linux_musl_x86_64.so',
    'x86_64-unknown-linux-musl/release/libstatsig_ffi.so.sig':
      'linux_musl_x86_64.so.sig',
  },
  'go-server-core-binaries-macos': {
    // AARCH64
    'aarch64-apple-darwin/release/libstatsig_ffi.dylib': 'macos_aarch64.dylib',
    'aarch64-apple-darwin/release/libstatsig_ffi.dylib.sig':
      'macos_aarch64.dylib.sig',
    // X86_64
    'x86_64-apple-darwin/release/libstatsig_ffi.dylib': 'macos_x86_64.dylib',
    'x86_64-apple-darwin/release/libstatsig_ffi.dylib.sig':
      'macos_x86_64.dylib.sig',
  },
};

export async function publishGo(options: PublisherOptions) {
  Log.stepBegin('Configuration');
  const version = getRootVersion();
  const commitHash = await getCurrentCommitHash();
  Log.stepProgress(`Commit Hash: ${commitHash}`);
  Log.stepEnd(`Version: ${version}`);

  await cloneBinaryRepos();

  await moveBinaryFiles(options);

  await commitAndPush(version);
}

async function cloneBinaryRepos() {
  Log.stepBegin('Cloning Go Binary Repos');

  ensureEmptyDir(TEMP_PATH);

  await checkoutGoBinaryRepo('go-server-core-binaries-linux-gnu');
  await checkoutGoBinaryRepo('go-server-core-binaries-linux-musl');
  await checkoutGoBinaryRepo('go-server-core-binaries-macos');

  Log.stepEnd('Cloned Go Binary Repos');
}

async function checkoutGoBinaryRepo(repo: string) {
  Log.stepProgress(`Cloning Repo:: ${repo}`);

  const result = spawnSync(
    'git',
    ['clone', `https://github.com/statsig-io/${repo}`, `${TEMP_PATH}/${repo}`],
    { stdio: 'ignore' },
  );

  if (result.status !== 0) {
    throw new Error(`Failed to clone repo: ${repo}`);
  }

  Log.stepProgress(`Successfully cloned Repo: ${repo}`);
}

async function moveBinaryFiles(options: PublisherOptions) {
  Log.stepBegin('Moving Binary Files');

  const libFiles = [
    ...listFiles(options.workingDir, '**/target/**/release/*.dylib'),
    ...listFiles(options.workingDir, '**/target/**/release/*.so'),
    ...listFiles(options.workingDir, '**/target/**/release/*.dll'),
    ...listFiles(options.workingDir, '**/target/**/release/*.sig'),
  ].filter((file) => !file.includes('windows'));

  const missedFiles = [];

  Object.entries(ASSET_MAPPING).flatMap(([repo, value]) => {
    return Object.entries(value).map(([source, destination]) => {
      const foundIndex = libFiles.findIndex((file) => file.includes(source));
      if (foundIndex === -1) {
        missedFiles.push(`${repo} - ${source} - ${destination}`);
        return;
      }

      const found = libFiles[foundIndex];
      libFiles.splice(foundIndex, 1);

      execSync(`cp ${found} ${path.resolve(TEMP_PATH, repo, destination)}`);
    });
  });

  if (libFiles.length > 0) {
    Log.stepProgress(`Unused files: ${libFiles.join('\n - ')}`, 'failure');
    throw new Error('Failed to move all files');
  }

  if (missedFiles.length > 0) {
    Log.stepEnd(
      `Failed to move all files: ${missedFiles.join('\n - ')}`,
      'failure',
    );
    throw new Error('Failed to move all files');
  }

  Log.stepEnd('Moved Binary Files');
}

async function commitAndPush(version: SemVer) {
  Log.stepBegin('Committing and Pushing');

  await commitAndPushToRepo(version, 'go-server-core-binaries-linux-gnu');
  await commitAndPushToRepo(version, 'go-server-core-binaries-linux-musl');
  await commitAndPushToRepo(version, 'go-server-core-binaries-macos');

  Log.stepEnd('Committed and Pushed');
}

async function commitAndPushToRepo(version: SemVer, repo: string) {
  Log.stepProgress(`Adding ${version.toString()} tag to ${repo}`);

  const versionString = 'v' + version.toString();

  const git = simpleGit(path.resolve(TEMP_PATH, repo));
  await git.add('.');
  await git.commit(`chore: update binaries to version ${versionString}`);
  await git.tag(['-d', versionString]);
  await git.addTag(versionString);

  await git.push(['origin', versionString, '--force']);

  Log.stepProgress(`${repo} tagged as ${versionString}`);
}
