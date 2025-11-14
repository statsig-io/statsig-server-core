import { getRootedPath } from '@/utils/file_utils.js';
import fs from 'fs';

import { CommandBase } from './command_base.js';

export class SyncTestData extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Sync test data');
  }

  override async run() {
    await downloadJsonAndProtoFor(process.env.test_api_key!, 'eval_proj_dcs');

    await downloadJsonAndProtoFor(process.env.PERF_SDK_KEY!, 'perf_proj_dcs');

    await downloadJsonAndProtoFor(
      process.env.statsig_demo_server_key!,
      'demo_proj_dcs',
    );
  }
}

async function downloadJsonAndProtoFor(sdkKey: string, name: string) {
  downloadFromUrlToFile(
    `https://staging.statsigapi.net/v2/download_config_specs/${sdkKey}.json`,
    `${name}.json`,
  );
  downloadFromUrlToFile(
    `https://staging.statsigapi.net/v2/download_config_specs/${sdkKey}.json?supports_proto=1`,
    `${name}.pb.br`,
  );
}

async function downloadFromUrlToFile(url: string, filename: string) {
  const response = await fetch(url);
  const data = await response.text();
  writeToDataDir(filename, data);
}

function writeToDataDir(filename: string, data: string) {
  const path = getRootedPath(`statsig-rust/tests/data/${filename}`);
  fs.writeFileSync(path, data);
}
