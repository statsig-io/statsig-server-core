import { ensureEmptyDir, getRootedPath } from '@/utils/file_utils.js';
import {
  createEmptyRepository,
  getCurrentCommitHash,
  tryApplyGitConfig,
} from '@/utils/git_utils.js';
import {
  GhRelease,
  createAndMergeVersionBumpPullRequest,
  createReleaseForVersion,
  deleteReleaseAssetWithName,
  getBranchByVersion,
  getOctokit,
  getReleaseByVersion,
  uploadReleaseAsset,
} from '@/utils/octokit_utils.js';
import { zipAndMoveAssets } from '@/utils/publish_utils.js';
import { mapAssetsToTargets } from '@/utils/publish_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/terminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { execSync } from 'node:child_process';
import path from 'node:path';
import { Octokit } from 'octokit';
import { simpleGit } from 'simple-git';

import { PublisherOptions } from './publisher-options.js';

const CPP_REPO_NAME = 'statsig-cpp-core';
const TEMP_REPO_PATH = '/tmp/server-core-cpp';
const RENAMED_BINARY_MAP = {
  'centos7-x86_64-unknown-linux-gnu': 'x86_64-unknown-linux-gnu',
  'centos7-aarch64-unknown-linux-gnu': 'aarch64-unknown-linux-gnu',
  'x86_64-unknown-linux-musl': 'x86_64-unknown-linux-musl',
  'alpine-aarch64-unknown-linux-musl': 'aarch64-unknown-linux-musl',
};
export async function publishCpp(options: PublisherOptions) {
  Log.title(`Creating release for ${CPP_REPO_NAME}`);
  Log.stepBegin('Configuration');
  const version = getRootVersion();
  const commitHash = await getCurrentCommitHash();
  Log.stepProgress(`Commit Hash: ${commitHash}`);
  Log.stepEnd(`Version: ${version}`);

  await checkoutCurrentCppRepo(version);

  const octokit = await getOctokit();
  const repoPath = getRootedPath('statsig-cpp');

  await copyChangesToTempRepo(octokit, repoPath, version);
  const release = await createGithubReleaseForCppRepo(octokit, version);

  const mappedAssets = mapAssetsToTargets(options.workingDir);
  // Rename linux binaries so it's easier to find
  mappedAssets.forEach((asset, target) => {
    if (asset.assetName.includes('linux')) {
      let key = Object.keys(RENAMED_BINARY_MAP).find((k) => asset.assetName.includes(k));
      if (key) {
        asset.assetName = asset.assetName.replace(key, RENAMED_BINARY_MAP[key]);
      }
    }
  });
  const assetFiles = zipAndMoveAssets(mappedAssets, options.workingDir);

  Log.title('Uploading assets to release');

  for await (const asset of assetFiles) {
    await uploadLibFileToRelease(octokit, release, asset);
  }

  await createAndMergeVersionBumpPullRequest(
    octokit,
    CPP_REPO_NAME,
    version,
    version.toBranch(),
  );
}

async function verifyBranchDoesNotExist(octokit: Octokit, version: SemVer) {
  Log.stepBegin(`Checking if ${version.toBranch()} branch exists`);
  const foundBranch = await getBranchByVersion(octokit, CPP_REPO_NAME, version);

  if (foundBranch) {
    Log.stepEnd(`Branch ${version.toBranch()} already exists`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Branch ${version.toBranch()} does not exist`);
}

async function checkoutCurrentCppRepo(version: SemVer) {
  Log.stepBegin('Checking out current C++ repo');
  ensureEmptyDir(TEMP_REPO_PATH);
  Log.stepProgress(`Empty Repo Directory Created: ${TEMP_REPO_PATH}`);
  await createEmptyRepository(TEMP_REPO_PATH, CPP_REPO_NAME);

  const git = simpleGit(TEMP_REPO_PATH);
  await git.pull('origin', 'main');

  const commitResult = await git.log({ maxCount: 1 });
  const commit = commitResult.latest;
  Log.stepEnd(`Commit: ${commit.hash} ${commit.message}`);

  await git.checkoutLocalBranch(version.toBranch());
}

async function copyChangesToTempRepo(
  octokit: Octokit,
  repoPath: string,
  version: SemVer,
) {
  await verifyBranchDoesNotExist(octokit, version);

  Log.stepBegin('Copying C++ Changes');
  Log.stepProgress(`Source: ${repoPath}`);
  Log.stepProgress(`Destination: ${TEMP_REPO_PATH}`);
  execSync(`cp -r ${repoPath}/* ${TEMP_REPO_PATH}`);
  Log.stepEnd('Changes copied to temp repo');

  const git = simpleGit(TEMP_REPO_PATH);
  await tryApplyGitConfig(git);

  await git.add('.');
  await git.commit(`chore: sync changes from ${version.toString()}`);
  await git.push('origin', version.toBranch());

  Log.stepEnd(`Pushed changes to ${version.toBranch()}`);
}

async function createGithubReleaseForCppRepo(
  octokit: Octokit,
  version: SemVer,
): Promise<GhRelease> {
  await verifyReleaseDoesNotExist(octokit, version);

  Log.stepBegin('Creating release');

  const { result: newRelease, error } = await createReleaseForVersion(
    octokit,
    CPP_REPO_NAME,
    version,
  );

  if (!newRelease) {
    Log.stepEnd(`Failed to create release`, 'failure');
    console.error(error ?? 'Unknown error');
    process.exit(1);
  }

  Log.stepEnd(`Release created ${newRelease.html_url}`);

  return newRelease;
}

async function verifyReleaseDoesNotExist(octokit: Octokit, version: SemVer) {
  Log.stepBegin('Checking for existing release');
  const release = await getReleaseByVersion(octokit, CPP_REPO_NAME, version);

  if (release) {
    Log.stepProgress(
      `Failed to create release as one already exists`,
      'failure',
    );
    Log.stepProgress(`ID: ${release.id}`, 'failure');
    Log.stepProgress(`Upload URL: ${release.upload_url}`, 'failure');
    Log.stepEnd(`HTML URL: ${release.html_url}`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Release ${version} does not exist`);
}

async function uploadLibFileToRelease(
  octokit: Octokit,
  release: GhRelease,
  zipPath: string,
) {
  const name = path.basename(zipPath);
  Log.stepBegin('Attaching Asset to Release');

  Log.stepProgress(`Asset File: ${name}`);

  const didDelete = await deleteReleaseAssetWithName(
    octokit,
    CPP_REPO_NAME,
    release.id,
    name,
  );

  Log.stepProgress(
    didDelete ? 'Existing asset deleted' : 'No existing asset found',
  );

  const uploadUrl = release.upload_url;
  if (!uploadUrl) {
    Log.stepEnd('No upload URL found', 'failure');
    process.exit(1);
  }

  Log.stepProgress(`Release upload URL: ${uploadUrl}`);

  const { result, error } = await uploadReleaseAsset(
    octokit,
    CPP_REPO_NAME,
    release.id,
    zipPath,
    name,
  );

  if (error || !result) {
    const errMessage =
      error instanceof Error ? error.message : error ?? 'Unknown Error';

    Log.stepEnd(`Failed to upload asset: ${errMessage}`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Asset uploaded: ${result.browser_download_url}`);
}
