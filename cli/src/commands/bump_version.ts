import {
  commitAndPushChanges,
  getCurrentBranchName,
  getUpstreamRemoteForCurrentBranch,
} from '@/utils/git_utils.js';
import { SemVer } from '@/utils/semver.js';
import {
  Log,
  printStepBegin,
  printStepEnd,
  printTitle,
} from '@/utils/teminal_utils.js';
import { getRootVersion, setRootVersion } from '@/utils/toml_utils.js';
import chalk from 'chalk';
import { Command } from 'commander';

import { SyncVersion } from './sync_version.js';

type Options = {
  major: boolean;
  minor: boolean;
  patch: boolean;
  beta: boolean;
};

export class BumpVersion extends Command {
  constructor() {
    super('bump-version');

    this.description('Bump the version of the SDK');

    this.option('--major', 'Bump the major version');
    this.option('--minor', 'Bump the minor version');
    this.option('--patch', 'Bump the patch version');
    this.option('--beta', 'Bump the beta version');

    this.argument('[string]', 'The version to bump to');

    this.action(this.run.bind(this));
  }

  async run(providedVersion: string | undefined, options: Options) {
    printTitle('Bumping Version');

    printStepBegin('Getting current version');
    let version = getRootVersion();
    printStepEnd(`Version: ${chalk.blue(version.toString())}`);

    printStepBegin('Updating version');
    if (options.major) {
      version.major += 1;
      version.minor = 0;
      version.patch = 0;
      version.beta = 0;
    } else if (options.minor) {
      version.minor += 1;
      version.patch = 0;
      version.beta = 0;
    } else if (options.patch) {
      version.patch += 1;
      version.beta = 0;
    } else if (options.beta) {
      version.beta += 1;
    }

    if (providedVersion) {
      version = new SemVer(providedVersion);
    }

    printStepEnd(`Updated Version: ${version.toString()}`);

    printStepBegin('Writing version to cargo.toml');
    setRootVersion(version);
    printStepEnd(`Updated Version: ${version.toString()}`);

    SyncVersion.sync();

    Log.title('Commit and Push Changes');

    Log.stepBegin('Getting Branch Info');
    const branch = await getCurrentBranchName();
    const remoteBranch = version.toBranch();
    const remote = await getUpstreamRemoteForCurrentBranch();
    Log.stepProgress(`Local Branch: ${branch}`);
    Log.stepProgress(`Remote Branch: ${remoteBranch}`);
    Log.stepEnd(`Remote Name: ${remote}`);

    Log.stepBegin('Committing changes');
    const { success, error } = await commitAndPushChanges(
      '.',
      `chore: bump version to ${version.toString()}`,
      remote,
      branch,
      remoteBranch,
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
