import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { execSync, StdioOptions } from 'child_process';

import { BuilderOptions } from './builder-options.js';
import { Log } from '@/utils/terminal_utils.js';
const NIF_VERSION = "nif-2.15"
export function buildElixir(options: BuilderOptions) {
  options.subProject = 'statsig_elixir';
  if (options.os == 'windows') {
    // options.envSetupForBuild = 'set RUSTFLAGS="-C target-cpu=native" &&';
  } else {
    options.envSetupForBuild = 'RUSTFLAGS="-C target-feature=-crt-static"';
  }
  let buildcommand = `cargo build --release -p statsig_elixir --target-dir target/${options.target}`
  execAndLogSync(buildcommand);
  // Enforce dynamic library
  // Rename built .dylib file to be statsig_elixir-{version}-{target}
  // execSync('ls target -l', { cwd: BASE_DIR, stdio: 'inherit' });
  // execSync('ls target/release -l', { cwd: BASE_DIR, stdio: 'inherit' });

  let binPath = `target/release`;
  let isGHAction = process.env.GH_APP_ID != null;
  let version = execAndLogSync("cd statsig-rust && cargo pkgid | cut -d# -f2 | cut -d@ -f2", 'pipe').trim();
  if (isGHAction) {
    let renamedFile = `libstatsig_elixir-v${version}-${NIF_VERSION}-${options.target}.so`; // This is deliberately done, we need .so for Rustler
    let commandForDylib = `mv **/**/libstatsig_elixir.dylib ${binPath}/${renamedFile}`;
    let commandForSo = `mv **/**/libstatsig_elixir.so ${binPath}/${renamedFile}`;
    let commandForDll = `mv **/**/libstatsig_elixir.dll ${binPath}/${renamedFile}`;
    Log.stepBegin('Rename binary file');
    try {
      execAndLogSync(commandForDll);
      execAndLogSync(`ls . `);
      execAndLogSync(`echo "file-name=${renamedFile}" >> $GITHUB_OUTPUT`);
    } catch (e) {
      console.warn('Skip get dylib file ready: ' + e);
    }
    try {
      execAndLogSync(commandForDylib);
      execAndLogSync(`echo "file-name=${renamedFile}" >> $GITHUB_OUTPUT`);
    } catch (e) {
      console.warn('Skip get dylib file ready: ' + e);
    }
    try {
      execSync(commandForSo, { cwd: BASE_DIR, stdio: 'inherit' });
      execAndLogSync(`echo "file-name=${renamedFile}" >> $GITHUB_OUTPUT`);
    } catch (e) {
      console.warn('Skip get so file ready: ' + e);
    }
    console.log('after moved listing path files');
    Log.stepEnd('Rename binary file');
  }

  Log.stepEnd(`Built statsig-elixir`);
}

function execAndLogSync(command: string, stdio: StdioOptions = 'inherit'): string {
  Log.stepProgress(command);
  return execSync(command, { cwd: BASE_DIR, stdio: stdio })?.toString();
}
