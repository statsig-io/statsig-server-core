import { getRootedPath } from '@/utils/file_utils.js';
import {
  commitAndPushChanges,
  createEmptyRepository,
  getCurrentCommitHash,
} from '@/utils/git_utils.js';
import {
  createReleaseForVersion,
  getBranchByVersion,
  getOctokit,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { Octokit } from 'octokit';

import { PublisherOptions } from './publisher-options.js';

const PHP_REPO_NAME = 'statsig-core-php';

export async function publishPhp(options: PublisherOptions) {
  Log.stepBegin('Publishing PHP');

  const version = getRootVersion();
  const commitHash = await getCurrentCommitHash();
  Log.stepProgress(`Commit Hash: ${commitHash}`);
  Log.stepEnd(`Version: ${version}`);

  const octokit = await getOctokit();
  await pushChangesToPhpRepo(octokit, version);
  await createGithubRelaseForPhpRepo(octokit, version);

  Log.stepEnd('Publishing PHP');
}

async function pushChangesToPhpRepo(octokit: Octokit, version: SemVer) {
  Log.stepBegin('Pushing changes to GitHub');

  const repoPath = getRootedPath('statsig-php');

  await verifyBranchDoesNotExist(octokit, version);
  await setupLocalPhpRepo(repoPath);

  Log.stepBegin('Getting Branch Info');
  const branch = 'master';
  const remoteBranch = version.toBranch();
  const remote = 'origin';
  Log.stepProgress(`Local Branch: ${branch}`);
  Log.stepProgress(`Remote Branch: ${remoteBranch}`);
  Log.stepEnd(`Remote Name: ${remote}`);

  Log.stepBegin('Committing changes');
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

async function createGithubRelaseForPhpRepo(octokit: Octokit, version: SemVer) {
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

  Log.stepEnd(`Release created: ${newRelease.html_url}`);
}

async function verifyReleaseDoesNotExist(octokit: Octokit, version: SemVer) {
  Log.stepBegin('Checking for existing release');
  const release = await getReleaseByVersion(octokit, PHP_REPO_NAME, version);

  if (release) {
    Log.stepEnd(`Release already exists: ${release.html_url}`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Release ${version} does not exist`);
}
