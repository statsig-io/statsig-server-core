import { existsSync, rmSync } from 'fs';
import { simpleGit } from 'simple-git';

import { BASE_DIR, getRootedPath } from './file_utils.js';
import { getInstallationToken } from './octokit_utils.js';

export async function getCurrentCommitHash(): Promise<string> {
  const git = simpleGit(BASE_DIR);
  return git.revparse(['--short', 'HEAD']);
}

export async function createEmptyRepository(
  repoPath: string,
  repoName: string,
) {
  rmPhpRepo();

  const token = await getInstallationToken();

  const git = simpleGit(repoPath);
  await git.init();
  await git.addRemote(
    'origin',
    `https://oauth2:${token}@github.com/statsig-io/${repoName}`,
  );
}

export async function getUpstreamRemoteForCurrentBranch() {
  const git = simpleGit(BASE_DIR);
  const upstream = await git.revparse(['--abbrev-ref', '@{u}']);
  return upstream.split('/')[0];
}

export async function getCurrentBranchName() {
  const git = simpleGit(BASE_DIR);
  const branch = await git.branch();
  return branch.current;
}

export function rmPhpRepo() {
  const repoPath = getRootedPath('statsig-php/.git');

  if (!existsSync(repoPath)) {
    return;
  }

  rmSync(repoPath, { recursive: true, force: true });
}

export async function commitAndPushChanges(
  repoPath: string,
  message: string,
  remote: string,
  localBranch: string,
  remoteBranch: string,
  shouldPushChanges: boolean,
) {
  try {
    const git = simpleGit(repoPath);

    if (process.env['CI']) {
      await git.addConfig('user.name', 'statsig-kong[bot]');
      await git.addConfig(
        'user.email',
        'statsig-kong[bot]@users.noreply.github.com',
      );
    }

    await git.cwd(repoPath).add('.').commit(message);

    const status = await git.status();
    if (!status.isClean()) {
      throw new Error('There are unstaged changes');
    }

    if (shouldPushChanges) {
      await git.push(remote, `${localBranch}:${remoteBranch}`);
    }

    return { success: true, error: null };
  } catch (error) {
    return { success: false, error };
  }
}
