import { getRootedPath } from '@/utils/file_utils.js';
import {
  commitAndPushChanges,
  createEmptyRepository,
} from '@/utils/git_utils.js';
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

    Log.stepBegin('Creating empty repository');
    const repoPath = getRootedPath('statsig-php');
    await createEmptyRepository(repoPath, 'sigstat-php');
    Log.stepEnd(`Repo Created: ${repoPath}`);

    Log.stepBegin('Committing changes');
    const { success, error } = await commitAndPushChanges(
      repoPath,
      `chore: bump version to ${version.toString()}`,
      'origin',
      'master',
      version.toBranch(),
    );

    if (error || !success) {
      const errMessage =
        error instanceof Error ? error.message : error ?? 'Unknown Error';

      Log.stepEnd(`Failed to commit changes: ${errMessage}`, 'failure');
      return;
    }

    Log.stepEnd('Changes committed');
  }
}
