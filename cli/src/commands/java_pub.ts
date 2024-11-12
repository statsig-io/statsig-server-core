import { ensureEmptyDir } from '@/utils/file_utils.js';
import { getOctokit } from '@/utils/octokit_utils.js';
import {
  downloadAndUnzipAssets,
  getRelease,
  getStatsigLibAssets,
} from '@/utils/publish_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { Command } from 'commander';

const TEMP_DIR = '/tmp/statsig-java-build';

export class JavaPub extends Command {
  constructor() {
    super('java-pub');

    this.description('Publishes the statsig-java package to Maven');

    this.argument(
      '<repo>',
      'The name of the repository, e.g. private-statsig-server-core',
    );

    this.action(this.run.bind(this));
  }

  async run(repo: string) {
    Log.title('Publishing statsig-java to Maven');

    Log.stepBegin('Configuration');
    Log.stepEnd(`Repo: ${repo}`);

    const version = getRootVersion();
    const octokit = await getOctokit();

    ensureEmptyDir(TEMP_DIR);

    const release = await getRelease(octokit, repo, version);
    const assets = await getStatsigLibAssets(
      octokit,
      repo,
      release,
      'statsig-ffi-',
    );

    await downloadAndUnzipAssets(
      octokit,
      repo,
      assets,
      TEMP_DIR,
      true /* Extract With Name */,
    );
  }
}
