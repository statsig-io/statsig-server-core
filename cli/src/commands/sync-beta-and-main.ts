import { getOctokit } from '@/utils/octokit_utils.js';

import { CommandBase } from './command_base.js';


const owner = "statsig-io";
const repo = "private-statsig-server-core";
const stableBranch = "stable";
const mainBranch = "main";

type Options = {};

export class SyncBetaAndMain extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Sync stable and main');
  }

  override async run() {
    const octokit = await getOctokit();
    // 1. Get the latest commit SHA of main
    const mainRef = await octokit.rest.git.getRef({
      owner,
      repo,
      ref: `heads/${mainBranch}`,
    });

    const mainSha = mainRef.data.object.sha;

    // 2. Get the tree of stable branch
    const stableRef = await octokit.rest.git.getRef({
      owner,
      repo,
      ref: `heads/${stableBranch}`,
    });

    const stableCommitSha = stableRef.data.object.sha;

    const stableCommit = await octokit.rest.git.getCommit({
      owner,
      repo,
      commit_sha: stableCommitSha,
    });

    // 3. Create a new commit on top of main with the same tree as stable
    const newCommit = await octokit.rest.git.createCommit({
      owner,
      repo,
      message: `Rebase ${stableBranch} onto ${mainBranch}`,
      tree: stableCommit.data.tree.sha,
      parents: [mainSha], // this effectively rebases
    });

    // 4. Update stable branch to point to the new commit
    await octokit.rest.git.updateRef({
      owner,
      repo,
      ref: `heads/${stableBranch}`,
      sha: newCommit.data.sha,
      force: true, // like --force-with-lease
    });

    console.log(`Rebased ${stableBranch} onto ${mainBranch} successfully!`);
  }
}
