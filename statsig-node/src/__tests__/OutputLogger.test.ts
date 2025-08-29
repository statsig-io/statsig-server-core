import * as fs from "node:fs";
import * as path from "node:path";
import { Statsig, StatsigOptions, StatsigUser } from "../../build/index.js";
import { MockOutputLogger } from "./MockOutputLogger";
import { MockScrapi } from "./MockScrapi";

describe("OutputLogger Usage", () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;
  let outputLogger: MockOutputLogger;
  let outputLoggerSpies: {
    initialize: jest.SpyInstance;
    debug: jest.SpyInstance;
    info: jest.SpyInstance;
    warn: jest.SpyInstance;
    error: jest.SpyInstance;
    shutdown: jest.SpyInstance;
  };
  beforeAll(async () => {
    scrapi = await MockScrapi.create();
    outputLogger = new MockOutputLogger();
    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        "../../../statsig-rust/tests/data/eval_proj_dcs.json"
      ),
      "utf8"
    );
    scrapi.mock("/v2/download_config_specs", dcs, {
      status: 200,
      method: "GET",
    });
    outputLoggerSpies = {
      initialize: jest.spyOn(outputLogger, "initialize"),
      debug: jest.spyOn(outputLogger, "debug"),
      info: jest.spyOn(outputLogger, "info"),
      warn: jest.spyOn(outputLogger, "warn"),
      error: jest.spyOn(outputLogger, "error"),
      shutdown: jest.spyOn(outputLogger, "shutdown"),
    };
    const specsUrl = scrapi.getUrlForPath("/v2/download_config_specs");
    const logEventUrl = scrapi.getUrlForPath("/v1/log_event");
    const options: StatsigOptions = {
      specsUrl,
      logEventUrl,
      outputLoggerProvider: outputLogger,
      outputLogLevel: "debug",
      specsSyncIntervalMs: 1,
      eventLoggingFlushIntervalMs: 1,
    };
    statsig = new Statsig("secret-123", options);
    await statsig.initialize();
    statsig.checkGate(StatsigUser.withUserID("test-user"), "test-gate");
    statsig.logEvent(StatsigUser.withUserID("b-user"), "my_event");
  }, 10000);

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  it("calls initialize during SDK initialization", () => {
    expect(outputLoggerSpies.initialize).toHaveBeenCalled();
  });
  it("logs debug messages during SDK operations", () => {
    expect(outputLoggerSpies.debug).toHaveBeenCalledWith(
      expect.any(String),
      expect.any(String)
    );
  });
});
