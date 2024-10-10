const {
  load,
  DataType,
  open,
  wrapPointer,
  createPointer,
  funcConstructor,
  unwrapPointer,
  freePointer,
  PointerType,
} = require('ffi-rs');

const LIBRARY_NAME = 'statsig_ffi';
open({
  library: LIBRARY_NAME,
  path: '/Users/danielloomb/Projects/kong/bridges/core-napi-bridge/sdk/target/release/libstatsig_ffi.dylib',
});

const POT_TYPE = {
  pointer: DataType.I32,
};

function test_create() {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'test_create',
    retType: DataType.External,
    paramsType: [],
    paramsValue: [],
  });

  return ref;
}

const pot = test_create();
console.log('[JS]: test_create', pot);

function test_value(pot) {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'test_value',
    retType: DataType.Void,
    paramsType: [DataType.External],
    paramsValue: [pot],
  });

  return ref;
}

test_value(pot);

function test_mut_star(pot) {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'test_mut_star',
    retType: DataType.Void,
    paramsType: [DataType.External],
    paramsValue: [pot],
  });

  return ref;
}

const potWrapped = wrapPointer([pot])[0];

test_mut_star(potWrapped);

function statsig_options_create() {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'statsig_options_create',
    retType: DataType.External,
    paramsType: [],
    paramsValue: [],
  });

  return ref;
}

function statsig_options_release(optionsRef) {
  const ptr = wrapPointer([optionsRef])[0];
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'statsig_options_release',
    retType: DataType.Void,
    paramsType: [DataType.External],
    paramsValue: [ptr],
  });

  return ref;
}

const options = statsig_options_create();
console.log('[JS]: statsig_options_create', options);

function statsig_create(sdkKey, optionsRef) {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'statsig_create',
    retType: DataType.External,
    paramsType: [DataType.String, DataType.External],
    paramsValue: [sdkKey, optionsRef],
  });

  return ref;
}

const statsig = statsig_create(
  process.env.test_api_key,
  options,
);
console.log('[JS]: statsig_create', statsig);

function statsig_release(statsigRef) {
  const ptr = wrapPointer([statsigRef])[0];
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'statsig_release',
    retType: DataType.Void,
    paramsType: [DataType.External],
    paramsValue: [ptr],
  });

  return ref;
}

function statsig_initialize(statsigRef) {
  return new Promise((resolve) => {
    const signature = funcConstructor({
      paramsType: [],
      retType: DataType.I32,
    });

    const callback = () => {
      resolve();
      freePointer({
        paramsType: [signature],
        paramsValue: funcExternal,
        pointerType: PointerType.RsPointer,
      });
      return 0;
    };

    const funcExternal = createPointer({
      paramsType: [signature],
      paramsValue: [callback],
    });

    return load({
      library: LIBRARY_NAME,
      funcName: 'statsig_initialize',
      retType: DataType.Void,
      paramsType: [DataType.External, DataType.External],
      paramsValue: [statsigRef, unwrapPointer(funcExternal)[0]],
    });
  });
}

function statsig_user_create(userID, email) {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'statsig_user_create',
    retType: DataType.External,
    paramsType: [DataType.String, DataType.String],
    paramsValue: [userID, email],
  });

  return ref;
}

function statsig_user_release(userRef) {
  const ptr = wrapPointer([userRef])[0];
  const ref = load({
    library: LIBRARY_NAME,
    funcName: 'statsig_user_release',
    retType: DataType.Void,
    paramsType: [DataType.External],
    paramsValue: [ptr],
  });

  return ref;
}

function statsig_get_client_init_response(statsigRef, userRef) {
  return load({
    library: LIBRARY_NAME,
    funcName: 'statsig_get_client_init_response',
    retType: DataType.String,
    paramsType: [DataType.External, DataType.External],
    paramsValue: [statsigRef, userRef],
  });
}

function statsig_check_gate(statsigRef, userRef, gateName) {
  return load({
    library: LIBRARY_NAME,
    funcName: 'statsig_check_gate',
    retType: DataType.Boolean,
    paramsType: [DataType.External, DataType.External, DataType.String],
    paramsValue: [statsigRef, userRef, gateName],
  });
}

const gates = [];

statsig_initialize(statsig).then(() => {
  const user = statsig_user_create('dan', 'daniel@statsig.com');

  const allStart = performance.now();
  const times = {};

  gates.forEach((gate) => {
    const start = performance.now();
    let result = false;
    for (let i = 0; i < 1000; i++) {
      result = statsig_check_gate(statsig, user, gate);
    }
    const end = performance.now();
    times[gate] = end - start;
  });

  console.log(times);

  const allEnd = performance.now();
  console.log('all duration', allEnd - allStart);

  statsig_options_release(options);
  statsig_release(statsig);
  statsig_user_release(user);
});
