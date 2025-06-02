import { getRootedPath } from '@/utils/file_utils.js';
import {
  commitAndPushChanges,
  createEmptyRepository,
  getCurrentCommitHash,
} from '@/utils/git_utils.js';
import {
  GhRelease,
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
import path from 'node:path';
import { Octokit } from 'octokit';

import { PublisherOptions } from './publisher-options.js';

const PHP_REPO_NAME = 'statsig-php-core';

export async function publishPhp(options: PublisherOptions) {
  Log.title(`Creating release for ${PHP_REPO_NAME}`);

  Log.stepBegin('Configuration');
  const version = getRootVersion();
  const commitHash = await getCurrentCommitHash();
  Log.stepProgress(`Commit Hash: ${commitHash}`);
  Log.stepEnd(`Version: ${version}`);

  const octokit = await getOctokit();
  await pushChangesToPhpRepo(octokit, version);
  const release = await createGithubRelaseForPhpRepo(octokit, version);

  const mappedAssets = mapAssetsToTargets(options.workingDir);
  const assetFiles = zipAndMoveAssets(mappedAssets, options.workingDir);

  Log.title('Uploading assets to release');

  for await (const asset of assetFiles) {
    await uploadLibFileToRelease(octokit, release, asset);
  }
}

async function pushChangesToPhpRepo(octokit: Octokit, version: SemVer) {
  Log.stepBegin('Pushing changes to GitHub');

  const repoPath = getRootedPath('statsig-php');
  const branch = 'master';
  const remoteBranch = version.toBranch();
  const remote = 'origin';
  const args = {
    repoPath,
    message: `chore: bump version to ${version.toString()}`,
    remote,
    localBranch: branch,
    remoteBranch,
    shouldPushChanges: true,
    tag: version.toString(),
  };
  Log.stepProgress(`Tag: ${args.tag}`);
  Log.stepProgress(`Remote: ${args.remote}`);
  Log.stepProgress(`Local Branch: ${args.localBranch}`);
  Log.stepProgress(`Remote Branch: ${args.remoteBranch}`);
  Log.stepProgress(`Should Push Changes: ${args.shouldPushChanges}`);
  Log.stepEnd(`Remote Name: ${remote}`);

  await verifyBranchDoesNotExist(octokit, version);
  await setupLocalPhpRepo(repoPath);

  Log.stepBegin('Committing changes');

  const { success, error } = await commitAndPushChanges(args);

  if (error || !success) {
    const errMessage =
      error instanceof Error ? error.message : error ?? 'Unknown Error';

    Log.stepEnd(`Failed to commit changes: ${errMessage}`, 'failure');
    process.exit(1);
  }

  Log.stepEnd('Changes committed');
}

async function verifyBranchDoesNotExist(octokit: Octokit, version: SemVer) {
  Log.stepBegin(`Checking if ${version.toBranch()} branch exists`);
  const foundBranch = await getBranchByVersion(octokit, PHP_REPO_NAME, version);

  if (foundBranch) {
    Log.stepEnd(`Branch ${version.toBranch()} already exists`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Branch ${version.toBranch()} does not exist`);
}

async function setupLocalPhpRepo(repoPath: string) {
  Log.stepBegin(`Creating local ${PHP_REPO_NAME} repository`);
  await createEmptyRepository(repoPath, PHP_REPO_NAME);
  Log.stepEnd(`Repo Created: ${repoPath}`);
}

async function createGithubRelaseForPhpRepo(
  octokit: Octokit,
  version: SemVer,
): Promise<GhRelease> {
  await verifyReleaseDoesNotExist(octokit, version);

  Log.stepBegin('Creating release');

  const { result: newRelease, error } = await createReleaseForVersion(
    octokit,
    PHP_REPO_NAME,
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
  const release = await getReleaseByVersion(octokit, PHP_REPO_NAME, version);

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
    PHP_REPO_NAME,
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
    PHP_REPO_NAME,
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
