import { getCurrentBranchName, mergeToMainAndPush } from '@/utils/git_utils.js';
import {
  createPullRequestAgainstMain,
  getOctokit,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import { Octokit } from 'octokit';

import { CommandBase } from './command_base.js';

export class MergeToMain extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Merges the current branch to main',
    });
  }

  override async run() {
    Log.title('Merging to Main');

    const octokit = await getOctokit();
    const title = execSync('git log -1 --pretty=%B').toString().trim();
    const branch = await getCurrentBranchName();

    Log.stepBegin(`Creating pull request against main`);
    Log.stepProgress(`Title: ${title}`);
    Log.stepProgress(`Branch: ${branch}`);

    const pullRequest = await createPullRequestAgainstMain(octokit, {
      repository: 'private-statsig-server-core',
      title: `[Automated] ${title}`,
      body: 'Created and merged automatically by T.O.R.E',
      head: branch,
    });

    Log.stepEnd(`Created pull request ${pullRequest.html_url}`, 'success');

    Log.stepBegin(`Merging pull request`);
    Log.stepProgress(`Pull request number: ${pullRequest.number}`);

    const mergeResult = await mergePullRequest(octokit, pullRequest.number);

    Log.stepProgress(`Merge result: ${mergeResult.message}`);
    Log.stepEnd(`Merged pull request ${pullRequest.html_url}`, 'success');
  }
}

async function mergePullRequest(octokit: Octokit, prNumber: number) {
  const result = await octokit.rest.pulls.merge({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    pull_number: prNumber,
    merge_method: 'squash',
  });

  return result.data;
}
