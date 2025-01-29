import { getFilenameWithoutExtension } from '@/utils/file_utils.js';
import { Command } from 'commander';

export abstract class CommandBase extends Command {
  constructor(metaUrl: string) {
    super(getFilenameWithoutExtension(metaUrl));

    this.action(this.run.bind(this));
  }

  protected abstract run(..._args: any[]): void;
}
