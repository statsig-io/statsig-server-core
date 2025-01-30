import {
  Arch,
  OS,
  buildDockerImage,
  getArchInfo,
  getDockerImageTag,
} from '@/utils/docker_utils.js';
import { BASE_DIR } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import chalk from 'chalk';
import { exec } from 'child_process';

import { CommandBase } from './command_base.js';

const TEST_COMMANDS: Record<string, string> = {
  python: [
    'cd /app/statsig-pyo3',
    'maturin build',
    'pip install ../target/wheels/statsig_python_core*manylinux*.whl --force-reinstall',
    'python3 -m pytest tests --capture=no -v',
  ].join(' && '),
  java: [
    'cd /app',
    'cargo build -p statsig_ffi',
    'mkdir -p /app/statsig-ffi/bindings/java/src/main/resources/native',
    'cp target/debug/libstatsig_ffi.so /app/statsig-ffi/bindings/java/src/main/resources/native',
    'cd /app/statsig-ffi/bindings/java',
    './gradlew test --rerun-tasks --console rich',
  ].join(' && '),
  php: [
    'cd /app',
    'cargo build -p statsig_ffi',
    'cd /app/statsig-php',
    'composer update',
    'composer test',
  ].join(' && '),
  node: [
    'cd /app/statsig-node',
    'pnpm install',
    'pnpm exec napi build --cross-compile --platform --js index.js --dts index.d.ts --output-dir build',
    'pnpm test',
  ].join(' && '),
};

type Options = {
  skipDockerBuild: boolean;
  os: OS;
  arch: Arch;
};

export class UnitTests extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Run the tests for all relevant files');

    this.argument(
      '[language]',
      'The language to run tests for, e.g. python',
      'all',
    );

    this.option(
      '-sdb, --skip-docker-build',
      'Skip building the docker image',
      false,
    );

    this.option(
      '-os, --os <string>',
      'The OS to run tests for, e.g. debian',
      'debian',
    );

    this.option(
      '-a, --arch <string>',
      'The architecture to run tests for, e.g. amd64',
      'amd64',
    );
  }

  override async run(lang: string, options: Options) {
    Log.title('Running Tests');

    Log.stepBegin('Configuration');
    Log.stepProgress(`Language: ${lang}`);
    Log.stepEnd(`Skip Docker Build: ${options.skipDockerBuild}`);

    if (!options.skipDockerBuild) {
      buildDockerImage(options.os, options.arch);
    }

    const languages = lang === 'all' ? Object.keys(TEST_COMMANDS) : [lang];

    await Promise.all(
      languages.map(async (lang) => {
        await runTestInDockerImage(lang, options.os, options.arch);
      }),
    );

    Log.conclusion('Tests Ran');
  }
}

class BufferedOutput {
  private buffer: string[] = [];

  constructor(
    private tag: string,
    private kind: 'stdout' | 'stderr',
  ) {}

  add(line: string) {
    this.buffer.push(line);

    if (this.buffer.length >= 10) {
      this.flush();
    }
  }

  flush() {
    for (const line of this.buffer) {
      if (this.kind === 'stdout') {
        console.log(this.tag, line);
      } else {
        console.error(this.tag, line);
      }
    }

    this.buffer.length = 0;
  }
}

function runTestInDockerImage(lang: string, os: OS, arch: Arch): Promise<void> {
  const tag = chalk.blue(`[${lang}]`);

  const { docker } = getArchInfo(arch);
  const dockerImageTag = getDockerImageTag(os, arch);

  return new Promise((resolve, reject) => {
    Log.title(`Running tests for ${lang}`);
    const command = [
      'docker run --rm',
      `--platform ${docker}`,
      `-v "${BASE_DIR}":/app`,
      `-v "/tmp:/tmp"`,
      `-v "/tmp/statsig-server-core/cargo-registry:/usr/local/cargo/registry"`,
      dockerImageTag,
      `"${TEST_COMMANDS[lang]}"`, // && while true; do sleep 1000; done
    ].join(' ');

    Log.stepBegin(`${tag} Executing docker command for ${lang}`);
    Log.stepProgress(`${tag} ${command}`);

    const stdoutBuffer = new BufferedOutput(tag, 'stdout');
    const stderrBuffer = new BufferedOutput(tag, 'stderr');

    const child = exec(
      command,
      { cwd: BASE_DIR, shell: '/bin/bash' },
      (error, stdout, stderr) => {
        stdoutBuffer.flush();
        stderrBuffer.flush();

        if (error) {
          Log.stepEnd(`${tag} Tests failed for ${lang}`);
          reject(error);
        } else {
          Log.stepEnd(`${tag} Tests completed for ${lang}`);
          resolve();
        }
      },
    );

    child.stdout?.on('data', (data) => {
      stdoutBuffer.add(data.toString().trim());
    });

    child.stderr?.on('data', (data) => {
      stderrBuffer.add(data.toString().trim());
    });
  });
}
