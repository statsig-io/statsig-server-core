import { listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function publishPhp(options: PublisherOptions) {
  Log.stepBegin('Publishing PHP');

  Log.stepEnd('Publishing PHP');
}
