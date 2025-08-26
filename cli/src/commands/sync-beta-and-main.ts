import { getOctokit } from '@/utils/octokit_utils.js';

import { CommandBase } from './command_base.js';

const owner = 'statsig-io';
const repo = 'private-statsig-server-core';

export class SyncBetaAndMain extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Sync stable and main');
  }

  override async run() {
    const octokit = await getOctokit();
    await octokit.rest.repos.merge({
      owner,
      repo,
      base: 'stable', // branch to merge into
      head: 'main', // branch to merge from
      commit_message: '[Auto]Sync stable with main',
    });
    // TODO (xinli): Scan through if there is any uncommon changes

    // 2. Merge stable back into main
    await octokit.rest.repos.merge({
      owner,
      repo,
      base: 'main', // branch to merge into
      head: 'stable', // branch to merge from
      commit_message: '[Auto]Sync main with stable',
    });
  }
}
