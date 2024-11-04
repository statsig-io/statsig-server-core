/**
 * To demonstrate usage, requires u
 */

import { SpecAdapterConfig, AdapterType, LogLevel, Statsig, StatsigOptions, StatsigUser } from "@sigstat/sigstat-napi";
import test from "ava";
const { exec } = require('child_process');

const GRPC_SERVER_ADDRESS = ""

let statsig: Statsig;
test.before("setup", () => {
  exec('cargo run --bin mock_grpc_server', { cwd: './../statsig_grpc/' }, (error, stdout, stderr) => {})
  // To setup a local server
  // But the server will send a invalid json response
  const configs: SpecAdapterConfig[] = [
    {
      adapterType: AdapterType.NetworkGrpcWebsocket,
      specsUrl: GRPC_SERVER_ADDRESS,
      initTimeout: 3000,
    },
    {
      adapterType: AdapterType.NetworkHttp,
      specsUrl: "https://assetsconfigcdn.org/v2/download_config_specs",
      initTimeout: 3000,
    },
  ]
  let statsigOptions = new StatsigOptions(
    LogLevel.Debug,
    'prod',
    undefined,
    3000,
    undefined,
    undefined,
    undefined,
    configs
  )
  statsig = new Statsig("secret-key", statsigOptions);
})
test("", () => {
  statsig.initialize()
  let user = new StatsigUser("123", {})
  statsig.checkGate(user, "always_on_gate")
})