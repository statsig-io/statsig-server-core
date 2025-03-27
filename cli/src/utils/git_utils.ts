import { existsSync, rmSync } from 'fs';
import path from 'path';
import { SimpleGit, simpleGit } from 'simple-git';

import { BASE_DIR } from './file_utils.js';
import { getInstallationToken } from './octokit_utils.js';

export function getCurrentCommitHash(): Promise<string> {
  const git = simpleGit(BASE_DIR);
  return git.revparse(['--short', 'HEAD']);
}

export async function createEmptyRepository(
  repoPath: string,
  repoName: string,
) {
  removeDirectory(path.resolve(repoPath, '.git'));

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

export function removeDirectory(dir: string) {
  if (!existsSync(dir)) {
    return;
  }

  rmSync(dir, { recursive: true, force: true });
}

export async function createBranch(name: string, remote: string) {
  const git = simpleGit(BASE_DIR);
  await git.checkoutLocalBranch(name);
  await git.push(remote, name, ['--set-upstream']);
}

export async function mergeToMainAndPush() {
  const token = await getInstallationToken();
  const git = simpleGit(BASE_DIR);

  await tryApplyGitConfig(git);

  const authUrl = `https://oauth2:${token}@github.com/statsig-io/private-statsig-server-core`;

  await git.checkout('main');
  await git.pull(authUrl, 'main');
  await git.merge(['--no-ff', '-']);
  await git.push(authUrl, 'main');
}

export async function commitAndPushChanges(args: {
  repoPath: string;
  message: string;
  remote: string;
  localBranch: string;
  remoteBranch: string;
  shouldPushChanges: boolean;
  tag?: string;
}) {
  try {
    const git = simpleGit(args.repoPath);

    await tryApplyGitConfig(git);

    await git.cwd(args.repoPath).add('.').commit(args.message);

    const status = await git.status();
    if (!status.isClean()) {
      const noChangeError = new Error('There are unstaged changes');
      noChangeError.name = 'NoChangesError';
      throw noChangeError;
    }

    const options: string[] = [];

    if (args.tag) {
      await git.addTag(args.tag);

      options.push('--follow-tags');
    }

    if (args.shouldPushChanges) {
      await git.push(
        args.remote,
        `${args.localBranch}:${args.remoteBranch}`,
        options,
      );

      await git.pushTags(args.remote);
    }

    return { success: true, error: null };
  } catch (error) {
    return { success: false, error };
  }
}

async function tryApplyGitConfig(git: SimpleGit) {
  const isCI = process.env['CI'];

  if (isCI) {
    await git.addConfig('user.name', 'statsig-kong[bot]');
    await git.addConfig(
      'user.email',
      'statsig-kong[bot]@users.noreply.github.com',
    );
  }
}

//
