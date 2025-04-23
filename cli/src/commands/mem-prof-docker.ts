import {
  Arch,
  OPERATING_SYSTEMS,
  OS,
  buildDockerImage,
  getArchInfo,
  getDockerImageTag,
} from '@/utils/docker_utils.js';
import { ARCHITECTURES } from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { generateSvgChart } from '@/utils/svg_chart_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';
import { spawn } from 'node:child_process';
import fs from 'node:fs';

import { CommandBase } from './command_base.js';
import { PACKAGES } from './publishers/publisher-options.js';

type MemProfDockerOptions = {
  arch: Arch;
  os: OS;
};

export class MemProfDocker extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Runs a program in docker and profiles memory usage',
      options: [
        {
          flags: '-a, --arch <string>',
          description: 'The architecture to build for',
          choices: ARCHITECTURES,
        },
        {
          flags: '--os <string>',
          description: 'The operating system to build for',
          choices: OPERATING_SYSTEMS,
        },
      ],
      args: [
        {
          name: '<language>',
          description: 'The language run the profile on',
          choices: PACKAGES,
          required: true,
        },
      ],
    });
  }

  override async run(lang: string, options: MemProfDockerOptions) {
    options.arch ??= 'arm64';
    options.os ??= 'debian';

    buildDockerImage(options.os, options.arch);

    const { docker } = getArchInfo(options.arch);
    const tag = getDockerImageTag(options.os, options.arch);

    const actions = [
      'cd /app/examples/python/mem-prof',
      'pip install statsig-python-core',
      'python3 spec-sync.py',
    ].join(' && ');

    try {
      Log.stepBegin('Removing previous statsig-mem-prof container');
      execSync('docker rm -f statsig-mem-prof', { cwd: BASE_DIR });
      Log.stepEnd('Removed previous statsig-mem-prof container');
    } catch (e) {
      Log.stepEnd('No previous statsig-mem-prof container to remove');
    }

    const command = [
      'docker run -d',
      '--name statsig-mem-prof',
      `--platform ${docker}`,
      `-v "${BASE_DIR}":/app`,
      `-v "/tmp/statsig-server-core/root-cargo-registry:/root/.cargo/registry"`,
      tag,
      `"${actions}"`,
    ].join(' ');

    Log.stepBegin('Running Docker Command');
    Log.stepProgress(command);

    const containerId = execSync(command, { cwd: BASE_DIR }).toString().trim();
    Log.stepEnd(`Spawned Container: ${containerId}`);

    const poll = pollContainer(containerId);

    const wait = spawn('docker', ['wait', containerId]);

    wait.on('exit', () => {
      clearInterval(poll);

      const stats = fs.readFileSync(getRootedPath('docker_stats.csv'), 'utf8');
      const timeline = fs.readFileSync(
        getRootedPath('examples/python/mem-prof/timeline.csv'),
        'utf8',
      );
      const svg = generateSvgChart(stats, timeline);
      fs.writeFileSync(getRootedPath('docker_stats.svg'), svg);
    });
  }
}

function pollContainer(containerId: string) {
  const outpath = getRootedPath('docker_stats.csv');
  const file = fs.createWriteStream(outpath);
  file.write('timestamp,cpu_percent,mem_usage,mem_limit\n');

  return setInterval(
    () => {
      Log.stepBegin('Stats Poll');

      try {
        const stats = execSync(
          `docker stats --no-stream --format "{{.CPUPerc}},{{.MemUsage}}" ${containerId}`,
        )
          .toString()
          .trim();

        if (stats.includes('0.00%')) {
          Log.stepEnd('Container is not running');
          return;
        }

        const [cpu, mem] = stats.split(',');
        const [memUsage, memLimit] = mem.trim().split(' / ');

        const timestamp = Date.now();
        file.write(
          `${timestamp},${cpu.trim()},${memUsage.trim()},${memLimit.trim()}\n`,
        );
        Log.stepEnd(stats);
      } catch (e) {
        console.error(e);
      }
    },
    10, // because we use execSync, the actual time between polls is based on when the docker stats command is finished
  );
}
