import { Log } from '@/utils/teminal_utils.js';
import { Command } from 'commander';

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

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  async run(_repo: string) {
    Log.title('Publishing statsig-java to Maven');
  }
}
