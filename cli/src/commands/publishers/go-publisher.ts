import {
  ensureEmptyDir,
  getRootedPath,
  listFiles,
} from '@/utils/file_utils.js';
import { getCurrentCommitHash, tryApplyGitConfig } from '@/utils/git_utils.js';
import {
  createAndMergeVersionBumpPullRequest,
  getInstallationToken,
  getOctokit,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/terminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import chalk from 'chalk';
import { execSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { SimpleGit, simpleGit } from 'simple-git';

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

  await cloneAllRepos();

  // Commit Binary Repos First
  await moveBinaryFiles(options);
  await commitAndPushBinaryRepos(version);

  // Commit Go Core Repo Last
  await moveGoCoreFiles(options, version);
  await commitAndPushGoCoreRepo(version);
}

async function cloneAllRepos() {
  Log.stepBegin('Cloning Go Binary Repos');

  ensureEmptyDir(TEMP_PATH);

  await checkoutRepo('go-server-core-binaries-linux-gnu');
  await checkoutRepo('go-server-core-binaries-linux-musl');
  await checkoutRepo('go-server-core-binaries-macos');
  await checkoutRepo('statsig-go-core');

  Log.stepEnd('Cloned Go Binary Repos');
}

async function checkoutRepo(repo: string) {
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

async function moveGoCoreFiles(options: PublisherOptions, version: SemVer) {
  Log.stepBegin('Moving Go Core Files');

  const destPath = `${TEMP_PATH}/statsig-go-core`;
  const rootPath = getRootedPath('statsig-go');

  // Remove all files in the destination path
  execSync(`cd ${destPath} && rm -rf ./*`);

  execSync(`cp -r ${rootPath}/* ${destPath}`);

  updateGoVersion(version.toString(), `${destPath}/go.mod`);
}

function updateGoVersion(version: string, path: string) {
  Log.stepBegin('Updating go version');
  const contents = fs.readFileSync(path, 'utf8');
  const was = contents.match(
    /go-server-core-binaries-linux-gnu v([^\s]+)/,
  )?.[1];

  if (!was) {
    Log.stepEnd('No version found', 'failure');
    process.exit(1);
  }

  let updated = contents.replace(
    /go-server-core-binaries-linux-gnu v([^\s]+)/,
    `go-server-core-binaries-linux-gnu v${version}`,
  );
  updated = updated.replace(
    /go-server-core-binaries-linux-musl v([^\s]+)/,
    `go-server-core-binaries-linux-musl v${version}`,
  );
  updated = updated.replace(
    /go-server-core-binaries-macos v([^\s]+)/,
    `go-server-core-binaries-macos v${version}`,
  );
  fs.writeFileSync(path, updated, 'utf8');

  Log.stepEnd(`Updated Version: ${chalk.strikethrough(was)} -> ${version}`);
}

async function commitAndPushBinaryRepos(version: SemVer) {
  Log.stepBegin('Committing and Pushing');

  await commitAndPushToRepo(
    version,
    'go-server-core-binaries-linux-gnu',
    'tag',
  );
  await commitAndPushToRepo(
    version,
    'go-server-core-binaries-linux-musl',
    'tag',
  );
  await commitAndPushToRepo(version, 'go-server-core-binaries-macos', 'tag');

  Log.stepEnd('Committed and Pushed');
}

async function commitAndPushGoCoreRepo(version: SemVer) {
  Log.stepBegin('Committing and Pushing');

  const commitHash = await commitAndPushToRepo(
    version,
    'statsig-go-core',
    'tag-and-branch',
  );

  Log.stepEnd(`Committed and Pushed: ${commitHash}`);

  const octokit = await getOctokit();

  await createAndMergeVersionBumpPullRequest(
    octokit,
    'statsig-go-core',
    version,
    version.toBranch(),
  );

  Log.stepEnd('Committed and Pushed');
}

async function commitAndPushToRepo(
  version: SemVer,
  repo: string,
  mode: 'tag' | 'tag-and-branch',
) {
  Log.stepProgress(`Adding ${version.toString()} tag to ${repo}`);
  const token = await getInstallationToken();
  const authUrl = `https://token:${token}@github.com/statsig-io/${repo}`;

  const versionTag = 'v' + version.toString();

  const git = simpleGit(path.resolve(TEMP_PATH, repo));
  await tryApplyGitConfig(git);

  await git.addRemote('authed-origin', authUrl);

  if (mode === 'tag-and-branch') {
    await git.pull('authed-origin', 'main');

    const commitResult = await git.log({ maxCount: 1 });
    const commit = commitResult.latest;
    Log.stepEnd(`Commit: ${commit.hash} ${commit.message}`);

    await git.checkoutLocalBranch(version.toBranch());
  }

  await git.add('.');
  await git.commit(`chore: update binaries to version ${versionTag}`);
  await tryGitDelete(git, versionTag);
  await git.addTag(versionTag);

  await git.push(['authed-origin', versionTag, '--force']);

  if (mode === 'tag-and-branch') {
    await git.push('authed-origin', version.toBranch(), ['--force']);
  }

  Log.stepProgress(`${repo} tagged as ${versionTag}`);
}

async function tryGitDelete(git: SimpleGit, tag: string) {
  try {
    const tags = await git.tags();
    if (tags.all.includes(tag)) {
      await git.tag(['-d', tag]);
    }
  } catch (error) {
    console.error(`Failed to delete tag ${tag}: ${error}`);
  }
}
