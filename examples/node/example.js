import { Statsig, StatsigOptions, StatsigUser } from "statsig-napi";

const statsig = new Statsig(
  process.env.test_api_key,
  {}
);

console.log("PID: " + process.pid + " process title is " + process.title);

class Foo {
  bar1 = "a;lskdjfl;aksdjf;lkasjdfl;kajsdlfkjasdflas";
  bar2 =
    "a;lskdjfl;aksdjf;lkasjdfl;kajsdlfkjasdflaskajsdlfkjasdflaskajsdlfkjasdflaskajsdlfkjasdflas";
  bar3 = "a;lskdjfl;aksdjf;lkasjdfl;kajsdlfkjasdflas";
  bar4 = "a;lskdjfl;aksdjf;lkasjdfl;kajsdlfkjasdflas";
  bar5 = "a;lskdjfl;aksdjf;lkasjdfl;kajsdlfkjasdflas";
}

let work;
work = () => {
  for (let i = 0; i < 1000; i++) {
    const user = new StatsigUser("a-user");
    // const options = new StatsigOptions();
    // const foo = new Foo();
  }

  if (global.gc) {
    global.gc();
  }
  setTimeout(() => work(), 1000);
};

work();

// statsig.initialize().then(() => {
//   // const user = new StatsigUser("a-user");
//   // const config = statsig.getDynamicConfig(user, "big_number");
//   // console.log("GetConfig", config);

//   // const gateVal = statsig.checkGate(user, "test_public");
//   // console.log("CheckGate", gateVal);

//   // const gate = statsig.getFeatureGate(user, "test_public");
//   // console.log("GetFeatureGate", gate);

//   // const experiment = statsig.getExperiment(user, "exp_with_obj_and_array");
//   // console.log("GetExperiment", experiment);

//   const start = performance.now();
//   let result = "";
//   for (let i = 0; i < 1000; i++) {
//     const user = new StatsigUser("user_" + i, "daniel@statsig.com");
//     result = statsig.getClientInitializeResponse(user);
//   }
//   const end = performance.now();

//   console.log(result);
//   console.log(end - start);
// });
