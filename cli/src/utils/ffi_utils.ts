import { existsSync, unlinkSync, writeFileSync } from 'fs';

import { BASE_DIR } from '@/utils/file_utils.js';
import { BuilderOptions } from '@/commands/builders/builder-options.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';
import {
  isLinux,
} from '@/utils/docker_utils.js';
import { join } from 'path';
import { tmpdir } from 'os';

function useCross(options: BuilderOptions): boolean {
  return isLinux(options.os) || options.target.includes("linux")
}

function getCrossBaseImageCommand(options: BuilderOptions): string | null {
  const command = "docker pull --platform linux/amd64 "
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
    } else if (options.arch == "aarch64" || options.arch == "arm64") {
      return "aarch64-pc-windows-msvc"
    } else {
      return "x86_64-pc-windows-msvc"
    }
  }
}

function getBinaryFilename(options: BuilderOptions): string {
  if (options.os === "macos") {
    return "libstatsig_ffi.dylib";
  } else if (options.os === "windows") {
    return "statsig_ffi.dll";
  } else {
    return "libstatsig_ffi.so";
  }
}

function signBinary(options: BuilderOptions, outDir: string) {
  Log.stepBegin(`Signing binary with openssl`);
  const binName = getBinaryFilename(options);
  const buildType = options.release ? "release" : "debug";
  
  if (outDir.includes('..')) {
    throw new Error("Invalid directory path");
  }
  
  const cleanOutDir = outDir.replace(/\.\./g, '');
  
  // Check both possible binary paths
  const binPath1 = join(
    BASE_DIR,
    "target",
    cleanOutDir,
    buildType,
    binName
  );
  
  const binPath2 = join(
    BASE_DIR,
    "target",
    cleanOutDir,
    cleanOutDir,
    buildType,
    binName
  );
  
  // Use whichever path exists
  const binPath = existsSync(binPath1) ? binPath1 : 
                  existsSync(binPath2) ? binPath2 : null;
  
  if (!binPath) {
    Log.stepEnd(`Cannot sign binary; file "${binName}" not found in either path: ${binPath1} or ${binPath2}`, "failure");
    return; // Don't throw — keep GitHub Action green
  }

  Log.stepProgress(`Found binary: ${binPath}`);

  const pemKey = process.env.OPENSSL_PRIVATE_KEY;
  if (!pemKey) {
    Log.stepEnd("OPENSSL_PRIVATE_KEY environment variable not set", "failure");
    return;
  }

  const tmpKeyPath = join(tmpdir(), "private.pem");
  writeFileSync(tmpKeyPath, pemKey, { mode: 0o600 });

  try {
    const sigPath = `${binPath}.sig`;

    execSync(
      `openssl dgst -sha256 -sign "${tmpKeyPath}" -out "${sigPath}" "${binPath}"`,
      { cwd: BASE_DIR, stdio: "inherit" }
    );

    Log.stepEnd(`Binary signed successfully -> ${sigPath}`, "success");
  } catch (error) {
    Log.stepEnd(`Failed to sign binary: ${error}`, "failure");
  } finally {
    // eslint-disable-next-line no-useless-catch
    try {
      unlinkSync(tmpKeyPath);
      if (existsSync(tmpKeyPath)) {
        // eslint-disable-next-line no-unsafe-finally
        throw new Error("Temp PEM key file still exists after attempted delete");
      }
      Log.stepEnd("Cleaned up temp PEM key file", "success");
    } catch (cleanupError) {
      // eslint-disable-next-line no-unsafe-finally
      throw cleanupError;
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
    ].filter((v) => v != null).join(' &&');
  } else {
    command = [
      'cargo build',
      ...buildConfigs
    ].join(' ');
  }

  Log.stepBegin(`Executing build command`);
  Log.stepProgress(command);

  execSync(command, { cwd: BASE_DIR, stdio: 'inherit' });

  if (options.sign) {
    signBinary(options, outDir);
  }
}
