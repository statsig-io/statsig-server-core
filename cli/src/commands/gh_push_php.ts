import { getRootedPath } from '@/utils/file_utils.js';
import {
  commitAndPushChanges,
  createEmptyRepository,
} from '@/utils/git_utils.js';
import { getBranchByVersion, getOctokit } from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { Command } from 'commander';

export class GhPushPhp extends Command {
  constructor() {
    super('gh-push-php');

    this.description('Pushes the statsig-php package to GitHub');

    this.action(this.run.bind(this));
  }

  async run() {
    Log.title('Pushing statsig-php to GitHub');

    const version = getRootVersion();

    Log.stepBegin(`Checking if ${version.toBranch()} branch exists`);
    const octokit = await getOctokit();
    const foundBranch = await getBranchByVersion(
      octokit,
      'statsig-core-php',
      version,
    );

    if (foundBranch) {
      Log.stepEnd(`Branch ${version.toBranch()} already exists`, 'failure');
      process.exit(1);
    }
    Log.stepEnd(`Branch ${version.toBranch()} does not exist`);

    Log.stepBegin('Creating empty repository');
    const repoPath = getRootedPath('statsig-php');
    await createEmptyRepository(repoPath, 'statsig-core-php');
    Log.stepEnd(`Repo Created: ${repoPath}`);

    Log.stepBegin('Committing changes');

    Log.stepBegin('Getting Branch Info');
    const branch = 'master';
    const remoteBranch = version.toBranch();
    const remote = 'origin';
    Log.stepProgress(`Local Branch: ${branch}`);
    Log.stepProgress(`Remote Branch: ${remoteBranch}`);
    Log.stepEnd(`Remote Name: ${remote}`);

    Log.stepBegin('Committing changes');
    const { success, error } = await commitAndPushChanges(
      repoPath,
      `chore: bump version to ${version.toString()}`,
      remote,
      branch,
      version.toBranch(),
      true /* shouldPushChanges */,
    );

    if (error || !success) {
      const errMessage =
        error instanceof Error ? error.message : error ?? 'Unknown Error';

      Log.stepEnd(`Failed to commit changes: ${errMessage}`, 'failure');
      process.exit(1);
    }

    Log.stepEnd('Changes committed');

    Log.conclusion('Successfully pushed statsig-php to GitHub');
  }
}
