const Statsig = require("statsig-node");

const name = "Dan";
const email = "daniel@statsig.com";

const user = { userID: name, email };

Statsig.initialize(process.env.test_api_key).then(
  () => {
    const gate_name = "test_public";

    const res = Statsig.checkGate(user, gate_name);
    console.log(`Gate check ${res ? "passed" : "failed"}!`);

    let result = "{}";
    start = performance.now();
    for (let i = 0; i < 1000; i++) {
      result = Statsig.getClientInitializeResponse(user);
    }
    end = performance.now();

    console.log(result);
    console.log(end - start);
  }
);
