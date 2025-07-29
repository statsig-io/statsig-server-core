import { BuilderOptions } from '@/commands/builders/builder-options.js';
import {
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';

function useCross(options: BuilderOptions): boolean {
  return isLinux(options.os) || options.target.includes("linux")
}

function getCrossBaseImageCommand(options: BuilderOptions): string | null {
  let command = "docker pull --platform linux/amd64 "
  if (options.target == "aarch64-unknown-linux-gnu") {
    return command + 'ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main-centos'
  } else if (options.target == "x86_64-unknown-linux-gnu") {
    return command + 'ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5-centos'
  } else if (options.target == "aarch64-unknown-linux-musl") {
    return command + 'ghcr.io/cross-rs/aarch64-unknown-linux-musl:0.2.5'
  } else if (options.target == "x86_64-unknown-linux-musl") {
    return command + 'ghcr.io/cross-rs/x86_64-unknown-linux-musl:0.2.5'
  }
  return null
}

function getExtraBuildArgs(options: BuilderOptions): string {
  if (options.target == "aarch64-unknown-linux-gnu") {
    // When building within an older version we need this variable
    return 'CFLAGS="-D__ARM_ARCH=8" '
  }
  return ''
}

export function detectTarget(options: BuilderOptions): string {
  if (options.os == "alpine") {
    if (options.arch == "aarch64" || options.arch == "arm64") {
      return "aarch64-unknown-linux-musl";
    } else {
      return "x86_64-unknown-linux-musl";
    }
  } else if (options.os == "centos7" || options.os == "amazonlinux2" || options.os == "debian" || options.os == "amazonlinux2023") {
    if (options.arch == "aarch64" || options.arch == "arm64") {
      return "aarch64-unknown-linux-gnu";
    } else {
      return "x86_64-unknown-linux-gnu"
    }
  } else if (options.os == "macos") {
    if (options.arch == "aarch64" || options.arch == "arm64") {
      return "aarch64-apple-darwin"
    } else {
      return "x86_64-apple-darwin"
    }
  } else if (options.os == "windows") {
    if (options.arch == "x86") {
      return "i686-pc-windows-msvc"
    } else {
      return "x86_64-pc-windows-msvc"
    }
  }
}


export function buildFfiHelper(options: BuilderOptions) {
  if (options.target == null) {
    options.target = detectTarget(options)
    Log.info(`Not setting target, deriving target to be ${options.target}` )
  }
  const outDir = options.target;
  const shouldUseCross = useCross(options)
  const buildConfigs = [`-p statsig_ffi`,
    options.release ? '--release' : '',
    `--target-dir target/${outDir}`]
  let command = "";
  if (shouldUseCross) {
    buildConfigs.push(`--target ${options.target!}`)
    command = [
      'CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install cross --git https://github.com/cross-rs/cross',
      getCrossBaseImageCommand(options),
      `${getExtraBuildArgs(options)} cross build ${buildConfigs.join(' ')}`
    ].filter((v, i) => v != null).join(' &&');
  } else {
    command = [
      'cargo build',
      ...buildConfigs
    ].join(' ');
  }

  Log.stepBegin(`Executing build command`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });
}
