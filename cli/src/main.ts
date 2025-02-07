import { loadCommands } from '@/commands/index.js';
import { program } from 'commander';

program
  .name('tore')
  .version('0.0.1')
  .description('Tool for Organizing Rust Environments');

const commands = await loadCommands();

commands.forEach((command) => {
  program.addCommand(command);
});

program.parseAsync(process.argv).catch((e) => {
  console.error(e);
  process.exit(1);
});
