import { Commands } from '@/commands/index.js';
import { program } from 'commander';

program
  .name('run')
  .version('0.0.1')
  .description('Statsig Server Core Build Tool');

Commands.forEach((command) => {
  program.addCommand(command);
});

program.parseAsync(process.argv).catch((e) => {
  console.error(e);
  process.exit(1);
});
