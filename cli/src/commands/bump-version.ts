import { BASE_DIR } from '@/utils/file_utils.js';
import {
  commitAndPushChanges,
  createBranch,
  getCurrentBranchName,
  getUpstreamRemoteForCurrentBranch,
  setGitHubOutput,
} from '@/utils/git_utils.js';
import { SemVer } from '@/utils/semver.js';
import {
  Log,
  printConclusion,
  printStepBegin,
  printStepEnd,
  printTitle,
} from '@/utils/terminal_utils.js';
import { getRootVersion, setRootVersion } from '@/utils/toml_utils.js';
import chalk from 'chalk';
import { execSync } from 'child_process';

import { CommandBase } from './command_base.js';
import { SyncVersion } from './sync-version.js';

type Options = {
  major?: boolean;
  minor?: boolean;
  patch?: boolean;
  beta?: boolean;
  betaDate?: boolean;
  rc?: boolean;
  doNotPush?: boolean;
  createBranch?: boolean;
};

export class BumpVersion extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Bump the version of the SDK');

    this.option('--major', 'Bump the major version');
    this.option('--minor', 'Bump the minor version');
    this.option('--patch', 'Bump the patch version');
    this.option('--beta', 'Bump the beta version');
    this.option('--rc', 'Bump the rc version');
    this.option(
      '--beta-date',
      'Bump the beta version based on the current date',
    );
    this.option('--do-not-push', 'Do not push the changes to the remote');
    this.option('--create-branch', 'Create a new branch for the version');
    this.argument('[string]', 'The version to bump to');
  }

  override async run(providedVersion: string | undefined, options: Options) {
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
    } else if (options.betaDate) {
      if (version.beta === 0) {
        version.patch += 1;
      }
      version.rc = 0;
      version.beta = this.getDateVersion();
    } else if (options.rc) {
      version.beta = 0;
      version.rc = this.getDateVersion();
    }

    if (providedVersion) {
      version = new SemVer(providedVersion);
    }

    if (options.createBranch) {
      const isCI = process.env['CI'];
      const newBranch = version.toBranch();
      printStepBegin(`Creating Branch: ${newBranch}`);
      await createBranch(newBranch, isCI ? 'origin' : 'private');
      printStepEnd(`Successfully Created Branch: ${newBranch}`);

      printStepBegin(`Setting to github_output`);
      setGitHubOutput('version_branch', newBranch);
      setGitHubOutput('version', version.toString());
      printStepEnd(`Setting to github_output`);
    }

    printStepEnd(`Updated Version: ${version.toString()}`);

    printStepBegin('Writing version to cargo.toml');
    setRootVersion(version);
    printStepEnd(`Updated Version: ${version.toString()}`);

    printConclusion('Succesfully Bumped Root Version');

    await SyncVersion.sync();

    Log.title('Commit and Push Changes');

    Log.stepBegin('Getting Branch Info');
    const branch = await getCurrentBranchName();
    const remoteBranch = version.toBranch();
    const remote = await getUpstreamRemoteForCurrentBranch();
    Log.stepProgress(`Local Branch: ${branch}`);
    Log.stepProgress(`Remote Branch: ${remoteBranch}`);
    Log.stepEnd(`Remote Name: ${remote}`);

    Log.stepBegin('Committing changes');
    const { success, error } = await commitAndPushChanges({
      repoPath: BASE_DIR,
      message: `chore: bump version to ${version.toString()}`,
      remote,
      localBranch: branch,
      remoteBranch,
      shouldPushChanges: options.doNotPush !== true,
    });

    if (error || !success) {
      const errMessage =
        error instanceof Error ? error.message : error ?? 'Unknown Error';

      Log.stepEnd(`Failed to commit changes: ${errMessage}`, 'failure');
      process.exit(1);
    }

    Log.stepEnd('Changes committed');

    Log.conclusion('Successfully Committed and Pushed Changes');
  }

  getDateVersion() {
    const date = new Date();
    // ISO 8601 -> YYMMDDHHMM
    return parseInt(date.toISOString().replace(/[-T:]/g, '').slice(2, 12));
  }
}
