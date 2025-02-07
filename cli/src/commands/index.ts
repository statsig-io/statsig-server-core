import { listFiles } from '@/utils/file_utils.js';
import path from 'node:path';

import { CommandBase } from './command_base.js';

export async function loadCommands() {
  const dir = path.dirname(import.meta.url).replace('file://', '');
  const files = listFiles(dir, '*.ts', { maxDepth: 1 });
  const commands = [];

  for (const file of files) {
    if (file.endsWith('index.ts') || file.endsWith('command_base.ts')) {
      continue;
    }

    const mod = await import(file);

    Object.entries(mod).forEach(([key, value]) => {
      if ((value as any).prototype instanceof CommandBase) {
        commands.push(new (value as any)());
        return;
      }

      throw new Error(`Command ${key} does not extend CommandBase`);
    });
  }

  return commands;
}
