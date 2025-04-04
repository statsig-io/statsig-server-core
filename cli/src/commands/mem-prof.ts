import { BASE_DIR } from '@/utils/file_utils.js';
import { execSync } from 'child_process';

import { CommandBase } from './command_base.js';
import { Exec } from './exec.js';

export class MemProf extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Runs a memory profiler on the current branch',
    });
  }

  override async run() {
    const command = [
      'cd /app/examples/rust/perf-monitor',
      'cargo build --release',
      'rm -rf /tmp/mem-prof',
      'mkdir -p /tmp/mem-prof',
      'cp ./target/release/perf-monitor /tmp/mem-prof',
      'cd /tmp/mem-prof',
      'heaptrack ./perf-monitor',
      'mv *.gz mem-prof.gz',

      ...heaptrack('allocations'),
      ...heaptrack('leaked'),
      ...heaptrack('transient'),

      'rm -rf /app/mem-prof',
      'mv /tmp/mem-prof /app/mem-prof',
    ];

    const exec = new Exec();
    await exec.run([command.join(' && ')], {
      os: 'debian',
      skipDockerBuild: false,
    });

    openAllocationsFlamegraph();
  }
}

function heaptrack(type: 'allocations' | 'leaked' | 'transient') {
  const cost = type.replace('transient', 'temporary');
  const heaptrackGraphs = [
    `heaptrack_print mem-prof.gz`,
    `--flamegraph-cost-type ${cost}`,
    `--print-flamegraph flamegraph-${type}.txt`,
    `--print-histogram histogram.txt`,
    `--print-massif massif.txt`,
    `> /dev/null`,
  ].join(' ');

  const countname = type == 'leaked' ? 'bytes' : 'allocations';
  const flamegraphCmd = [
    `flamegraph.pl flamegraph-${type}.txt`,
    `--width=1920`,
    `--color=mem`,
    `--title="${type.toUpperCase()}"`,
    `--countname=${countname}`,
    `> flamegraph-${type}.svg`,
  ].join(' ');

  const commands = [heaptrackGraphs, flamegraphCmd];

  if (type == 'leaked') {
    const heaptrackLeaks = [
      `heaptrack_print mem-prof.gz`,
      `--print-leaks=yes > leaks.txt`,
    ].join(' ');
    commands.push(heaptrackLeaks);
  }

  if (type == 'allocations') {
    const heaptrackPeaks = [
      `heaptrack_print mem-prof.gz`,
      `--print-peaks=yes > peaks.txt`,
    ].join(' ');
    commands.push(heaptrackPeaks);
  }

  if (type == 'transient') {
    const heaptrackPeaks = [
      `heaptrack_print mem-prof.gz`,
      `--print-temporary=yes > transient.txt`,
    ].join(' ');
    commands.push(heaptrackPeaks);
  }

  return commands;
}

function openAllocationsFlamegraph() {
  try {
    execSync('open -Ra "Google Chrome"', { stdio: 'ignore' });
    execSync(`open -a "Google Chrome" mem-prof/flamegraph-allocations.svg`, {
      cwd: BASE_DIR,
    });
  } catch (e) {
    execSync('open -a Safari mem-prof/flamegraph-allocations.svg', {
      cwd: BASE_DIR,
    });
  }
}
