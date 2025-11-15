import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';

import { BuilderOptions } from './builder-options.js';
import { isLinux } from '@/utils/docker_utils.js';

export function buildCpp(options: BuilderOptions) {
  Log.title(`Building statsig-cpp`);
  options.subProject = 'statsig_ffi';
  if (options.os == 'macos') {
    options.envSetupForBuild = `RUSTFLAGS="-C link-args=-Wl,-install_name,@rpath/libstatsig_ffi.dylib"`;
  } else if (isLinux(options.os)) {
    options.envSetupForBuild = `RUSTFLAGS="-C link-args=-Wl,-soname,libstatsig_ffi.so"`;
  }
  buildFfiHelper(options);

  Log.stepEnd(`Built statsig-cpp`);
}
