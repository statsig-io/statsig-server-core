import * as fs from "node:fs";
import * as path from "node:path";

import { Statsig, StatsigOptions, StatsigUser } from "../../build/index.js";
import { MockScrapi } from "./MockScrapi";
describe("Statsig Singleton", () => {
  let scrapi: MockScrapi;
  let options: StatsigOptions;
  const user = StatsigUser.withUserID("a-user");

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

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

    scrapi.mock("/v1/log_event", '{"success": true}', {
      status: 202,
      method: "POST",
    });

    const specsUrl = scrapi.getUrlForPath("/v2/download_config_specs");
    const logEventUrl = scrapi.getUrlForPath("/v1/log_event");
    options = {
      specsUrl,
      logEventUrl,
    };
  });

  afterAll(async () => {
    scrapi.close();
  });

  it("Call no output should error out", async () => {
    const warnSpy = jest.spyOn(console, "warn").mockImplementation();
    Statsig.shared();
    expect(warnSpy).toHaveBeenCalledWith(
      "[Statsig] No shared instance has been created yet. Call newShared() before using it. Returning an invalid instance"
    );
  });

  it("Should get a instance", async () => {
    Statsig.newShared("secret-key", options);
    expect(Statsig.shared() != null);
    await Statsig.shared().initialize();
  });

  it("Should run properly", async () => {
    const experiment = Statsig.shared().getExperiment(
      user,
      "exp_with_obj_and_array"
    );
    expect(experiment.getEvaluationDetails()).toMatchObject({
      reason: "Network:Recognized",
      lcut: expect.any(Number),
      receivedAt: expect.any(Number),
    });
  });

  it("Double initialize will return invalid instance", async () => {
    const warnSpy = jest.spyOn(console, "warn").mockImplementation();
    Statsig.newShared("secret-key");
    expect(warnSpy).toHaveBeenCalledWith(
      "[Statsig] Shared instance has been created, call removeSharedInstance() if you want to create another one. Returning an invalid instance"
    );
  });

  it("Remove and recreate will work", async () => {
    await Statsig.shared().shutdown();
    Statsig.removeSharedInstance();
    expect(!Statsig.hasShared());
  });
});
