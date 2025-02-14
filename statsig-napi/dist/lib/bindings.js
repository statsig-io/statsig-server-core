// prettier-ignore
/* eslint-disable */
// @ts-nocheck
/* auto-generated by NAPI-RS */

const { createRequire } = require('node:module')
require = createRequire(__filename);

const { readFileSync } = require('node:fs');
let nativeBinding = null;
const loadErrors = [];

const isMusl = () => {
  let musl = false;
  if (process.platform === 'linux') {
    musl = isMuslFromFilesystem();
    if (musl === null) {
      musl = isMuslFromReport();
    }
    if (musl === null) {
      musl = isMuslFromChildProcess();
    }
  }
  return musl;
};

const isFileMusl = (f) => f.includes('libc.musl-') || f.includes('ld-musl-');

const isMuslFromFilesystem = () => {
  try {
    return readFileSync('/usr/bin/ldd', 'utf-8').includes('musl');
  } catch {
    return null;
  }
};

const isMuslFromReport = () => {
  const report =
    typeof process.report.getReport === 'function'
      ? process.report.getReport()
      : null;
  if (!report) {
    return null;
  }
  if (report.header && report.header.glibcVersionRuntime) {
    return false;
  }
  if (Array.isArray(report.sharedObjects)) {
    if (report.sharedObjects.some(isFileMusl)) {
      return true;
    }
  }
  return false;
};

const isMuslFromChildProcess = () => {
  try {
    return require('child_process')
      .execSync('ldd --version', { encoding: 'utf8' })
      .includes('musl');
  } catch (e) {
    // If we reach this case, we don't know if the system is musl or not, so is better to just fallback to false
    return false;
  }
};

function requireNative() {
  if (process.platform === 'android') {
    if (process.arch === 'arm64') {
      try {
        return require('./sigstat-napi.android-arm64.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-android-arm64');
      } catch (e) {
        loadErrors.push(e);
      }
    } else if (process.arch === 'arm') {
      try {
        return require('./sigstat-napi.android-arm-eabi.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-android-arm-eabi');
      } catch (e) {
        loadErrors.push(e);
      }
    } else {
      loadErrors.push(
        new Error(`Unsupported architecture on Android ${process.arch}`),
      );
    }
  } else if (process.platform === 'win32') {
    if (process.arch === 'x64') {
      try {
        return require('./sigstat-napi.win32-x64-msvc.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-win32-x64-msvc');
      } catch (e) {
        loadErrors.push(e);
      }
    } else if (process.arch === 'ia32') {
      try {
        return require('./sigstat-napi.win32-ia32-msvc.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-win32-ia32-msvc');
      } catch (e) {
        loadErrors.push(e);
      }
    } else if (process.arch === 'arm64') {
      try {
        return require('./sigstat-napi.win32-arm64-msvc.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-win32-arm64-msvc');
      } catch (e) {
        loadErrors.push(e);
      }
    } else {
      loadErrors.push(
        new Error(`Unsupported architecture on Windows: ${process.arch}`),
      );
    }
  } else if (process.platform === 'darwin') {
    try {
      return require('./sigstat-napi.darwin-universal.node');
    } catch (e) {
      loadErrors.push(e);
    }
    try {
      return require('@sigstat/sigstat-napi-darwin-universal');
    } catch (e) {
      loadErrors.push(e);
    }

    if (process.arch === 'x64') {
      try {
        return require('./sigstat-napi.darwin-x64.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-darwin-x64');
      } catch (e) {
        loadErrors.push(e);
      }
    } else if (process.arch === 'arm64') {
      try {
        return require('./sigstat-napi.darwin-arm64.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-darwin-arm64');
      } catch (e) {
        loadErrors.push(e);
      }
    } else {
      loadErrors.push(
        new Error(`Unsupported architecture on macOS: ${process.arch}`),
      );
    }
  } else if (process.platform === 'freebsd') {
    if (process.arch === 'x64') {
      try {
        return require('./sigstat-napi.freebsd-x64.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-freebsd-x64');
      } catch (e) {
        loadErrors.push(e);
      }
    } else if (process.arch === 'arm64') {
      try {
        return require('./sigstat-napi.freebsd-arm64.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-freebsd-arm64');
      } catch (e) {
        loadErrors.push(e);
      }
    } else {
      loadErrors.push(
        new Error(`Unsupported architecture on FreeBSD: ${process.arch}`),
      );
    }
  } else if (process.platform === 'linux') {
    if (process.arch === 'x64') {
      if (isMusl()) {
        try {
          return require('./sigstat-napi.linux-x64-musl.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-x64-musl');
        } catch (e) {
          loadErrors.push(e);
        }
      } else {
        try {
          return require('./sigstat-napi.linux-x64-gnu.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-x64-gnu');
        } catch (e) {
          loadErrors.push(e);
        }
      }
    } else if (process.arch === 'arm64') {
      if (isMusl()) {
        try {
          return require('./sigstat-napi.linux-arm64-musl.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-arm64-musl');
        } catch (e) {
          loadErrors.push(e);
        }
      } else {
        try {
          return require('./sigstat-napi.linux-arm64-gnu.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-arm64-gnu');
        } catch (e) {
          loadErrors.push(e);
        }
      }
    } else if (process.arch === 'arm') {
      if (isMusl()) {
        try {
          return require('./sigstat-napi.linux-arm-musleabihf.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-arm-musleabihf');
        } catch (e) {
          loadErrors.push(e);
        }
      } else {
        try {
          return require('./sigstat-napi.linux-arm-gnueabihf.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-arm-gnueabihf');
        } catch (e) {
          loadErrors.push(e);
        }
      }
    } else if (process.arch === 'riscv64') {
      if (isMusl()) {
        try {
          return require('./sigstat-napi.linux-riscv64-musl.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-riscv64-musl');
        } catch (e) {
          loadErrors.push(e);
        }
      } else {
        try {
          return require('./sigstat-napi.linux-riscv64-gnu.node');
        } catch (e) {
          loadErrors.push(e);
        }
        try {
          return require('@sigstat/sigstat-napi-linux-riscv64-gnu');
        } catch (e) {
          loadErrors.push(e);
        }
      }
    } else if (process.arch === 'ppc64') {
      try {
        return require('./sigstat-napi.linux-ppc64-gnu.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-linux-ppc64-gnu');
      } catch (e) {
        loadErrors.push(e);
      }
    } else if (process.arch === 's390x') {
      try {
        return require('./sigstat-napi.linux-s390x-gnu.node');
      } catch (e) {
        loadErrors.push(e);
      }
      try {
        return require('@sigstat/sigstat-napi-linux-s390x-gnu');
      } catch (e) {
        loadErrors.push(e);
      }
    } else {
      loadErrors.push(
        new Error(`Unsupported architecture on Linux: ${process.arch}`),
      );
    }
  } else {
    loadErrors.push(
      new Error(
        `Unsupported OS: ${process.platform}, architecture: ${process.arch}`,
      ),
    );
  }
}

nativeBinding = requireNative();

if (!nativeBinding || process.env.NAPI_RS_FORCE_WASI) {
  try {
    nativeBinding = require('./sigstat-napi.wasi.cjs');
  } catch (err) {
    if (process.env.NAPI_RS_FORCE_WASI) {
      loadErrors.push(err);
    }
  }
  if (!nativeBinding) {
    try {
      nativeBinding = require('@sigstat/sigstat-napi-wasm32-wasi');
    } catch (err) {
      if (process.env.NAPI_RS_FORCE_WASI) {
        loadErrors.push(err);
      }
    }
  }
}

if (!nativeBinding) {
  const plat = process.platform;
  const arch = process.arch;
  const message =
    '[Statsig]: Native library not found for ' +
    plat +
    '-' +
    arch +
    (isMusl() ? '-musl' : '');

  if (loadErrors.length > 0) {
    // TODO Link to documentation with potential fixes
    //  - The package owner could build/publish bindings for this arch
    //  - The user may need to bundle the correct files
    //  - The user may need to re-install node_modules to get new packages

    throw new Error(message, { cause: loadErrors });
  }
  throw new Error(message);
}

module.exports.AutoReleasingStatsigOptionsRef =
  nativeBinding.AutoReleasingStatsigOptionsRef;
module.exports.AutoReleasingStatsigRef = nativeBinding.AutoReleasingStatsigRef;
module.exports.AutoReleasingStatsigUserRef =
  nativeBinding.AutoReleasingStatsigUserRef;
module.exports.consoleLoggerInit = nativeBinding.consoleLoggerInit;
module.exports.LogLevel = nativeBinding.LogLevel;
module.exports.SpecsAdapterTypeNapi = nativeBinding.SpecsAdapterTypeNapi;
module.exports.statsigCheckGate = nativeBinding.statsigCheckGate;
module.exports.statsigCreate = nativeBinding.statsigCreate;
module.exports.statsigGetClientInitResponse =
  nativeBinding.statsigGetClientInitResponse;
module.exports.statsigGetCurrentValues = nativeBinding.statsigGetCurrentValues;
module.exports.statsigGetDynamicConfig = nativeBinding.statsigGetDynamicConfig;
module.exports.statsigGetExperiment = nativeBinding.statsigGetExperiment;
module.exports.statsigGetFeatureGate = nativeBinding.statsigGetFeatureGate;
module.exports.statsigGetLayer = nativeBinding.statsigGetLayer;
module.exports.statsigInitialize = nativeBinding.statsigInitialize;
module.exports.statsigLogDynamicConfigExposure =
  nativeBinding.statsigLogDynamicConfigExposure;
module.exports.statsigLogExperimentExposure =
  nativeBinding.statsigLogExperimentExposure;
module.exports.statsigLogGateExposure = nativeBinding.statsigLogGateExposure;
module.exports.statsigLogLayerParamExposure =
  nativeBinding.statsigLogLayerParamExposure;
module.exports.statsigLogNumValueEvent = nativeBinding.statsigLogNumValueEvent;
module.exports.statsigLogStringValueEvent =
  nativeBinding.statsigLogStringValueEvent;
module.exports.statsigOptionsCreate = nativeBinding.statsigOptionsCreate;
module.exports.statsigShutdown = nativeBinding.statsigShutdown;
module.exports.statsigUserCreate = nativeBinding.statsigUserCreate;
