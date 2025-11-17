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
    'application/json; charset=utf-8',
  );
  downloadFromUrlToFile(
    `https://staging.statsigapi.net/v2/download_config_specs/${sdkKey}.json?supports_proto=1`,
    `${name}.pb.br`,
    'application/octet-stream',
  );
}

async function downloadFromUrlToFile(
  url: string,
  filename: string,
  contentType: string,
) {
  const response = await fetch(url);

  if (response.headers.get('Content-Type') !== contentType) {
    throw new Error(
      `Expected content type ${contentType} but got ${response.headers.get(
        'Content-Type',
      )}`,
    );
  }

  const data =
    contentType === 'application/octet-stream'
      ? await response.arrayBuffer()
      : await response.text();

  writeToDataDir(filename, data);
}

function writeToDataDir(filename: string, data: string | ArrayBuffer) {
  const path = getRootedPath(`statsig-rust/tests/data/${filename}`);
  fs.writeFileSync(
    path,
    data instanceof ArrayBuffer ? Buffer.from(data) : data,
  );
}
