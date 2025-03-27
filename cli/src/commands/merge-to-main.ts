import { mergeToMainAndPush } from '@/utils/git_utils.js';
import { Log } from '@/utils/teminal_utils.js';

import { CommandBase } from './command_base.js';

export class MergeToMain extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Merges the current branch to main',
    });
  }

  override async run() {
    Log.title('Merging to Main');

    await mergeToMainAndPush();
  }
}
