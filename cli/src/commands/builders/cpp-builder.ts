import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';

export function buildCpp(options: BuilderOptions) {
  Log.title(`Building statsig-cpp`);
  options.subProject = 'statsig_ffi';
  buildFfiHelper(options);

  Log.stepEnd(`Built statsig-cpp`);
}
