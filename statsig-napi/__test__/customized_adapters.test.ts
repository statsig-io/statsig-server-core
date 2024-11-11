import {
  SpecAdapterConfig,
  SpecsAdapterType,
  LogLevel,
  Statsig,
  StatsigOptions,
  StatsigUser,
} from '@sigstat/sigstat-napi';
import test from 'ava';
const { exec } = require('child_process');

const GRPC_SERVER_ADDRESS = '';

let statsig: Statsig;
test.before('setup', () => {
  exec(
    'cargo run --bin mock_grpc_server',
    { cwd: './../statsig_grpc/' },
    (error, stdout, stderr) => {},
  );
  // To setup a local server
  // But the server will send a invalid json response
  const configs: SpecAdapterConfig[] = [
    {
      adapterType: SpecsAdapterType.NetworkGrpcWebsocket,
      specsUrl: GRPC_SERVER_ADDRESS,
      initTimeoutMs: 3000,
    },
    {
      adapterType: SpecsAdapterType.NetworkHttp,
      specsUrl: 'https://api.statsigcdn.com/v2/download_config_specs',
      initTimeoutMs: 3000,
    },
  ];
  let statsigOptions = new StatsigOptions(
    LogLevel.Debug,
    'prod',
    undefined,
    3000,
    undefined,
    undefined,
    undefined,
    configs,
  );
  statsig = new Statsig('secret-key', statsigOptions);
});
test('', () => {
  statsig.initialize();
  let user = new StatsigUser('123', {});
  statsig.checkGate(user, 'always_on_gate');
});
