import {
  createReleaseForVersion,
  getBranchByVersion,
  getOctokit,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import chalk from 'chalk';
import { Command } from 'commander';

export class GhCreateRelease extends Command {
  constructor() {
    super('gh-create-release');

    this.description('Creates a new release on GitHub');

    this.argument('<repository>', 'The repository to create the release for');
    this.action(this.run.bind(this));
  }

  async run(repository: string) {
    Log.title('Creating GitHub Release');

    const version = getRootVersion();

    Log.stepBegin('Configuration');
    Log.stepProgress(`Repository: ${repository}`);
    Log.stepEnd(`Release Tag: ${version}`);

    const octokit = await getOctokit();

    Log.stepBegin('Checking for existing release');
    const release = await getReleaseByVersion(octokit, repository, version);

    if (release) {
      Log.stepEnd(`Release already exists: ${release.html_url}`, 'failure');
      return;
    }

    Log.stepEnd(`Release ${version} does not exist`);

    Log.stepBegin('Checking if branch exists');
    const branch = await getBranchByVersion(octokit, repository, version);

    if (!branch) {
      Log.stepEnd(`Branch ${version.toBranch()} does not exist`, 'failure');
      return;
    }

    Log.stepEnd(`Branch ${branch.ref} exists`);

    Log.stepBegin('Creating release');

    const newRelease = await createReleaseForVersion(
      octokit,
      repository,
      version,
      branch.object.sha,
    );

    if (!newRelease) {
      Log.stepEnd(`Failed to create release`, 'failure');
      return;
    }

    Log.stepEnd(`Release created: ${newRelease.html_url}`);

    console.log(`✅ ${chalk.green(`Successfully created release ${version}`)}`);
  }
}
