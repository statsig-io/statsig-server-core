const REF_TYPE = {
  pointer: DataType.U64,
  ref_type: DataType.U8,
};

const THING_TYPE = {
  pointer: DataType.U64,
  ref_type: DataType.U64,
};

function statsig_options_create() {
  const ref = load({
    library: LIBRARY_NAME,
    funcName: "statsig_options_create",
    retType: DataType.External,
    paramsType: [],
    paramsValue: [],
    freeResultMemory: false,
  });

  return ref;
}

function ref_release(ref) {
  return load({
    library: LIBRARY_NAME,
    funcName: "ref_release",
    retType: DataType.Void,
    paramsType: [DataType.External],
    paramsValue: [ref],
  });
}

function statsig_create(sdkKey, optionsRef) {
  return load({
    library: LIBRARY_NAME,
    funcName: "statsig_create",
    retType: DataType.Void,
    paramsType: [DataType.String, DataType.External],
    paramsValue: [sdkKey, optionsRef],
  });
}

// let ref = test_thing();
let ref = statsig_options_create();
console.log("Opt Ref", ref);

// take_thing(ref);

const wrapPtr = wrapPointer([ref])[0];
const unwrapPtr = unwrapPointer([ref])[0];

// console.log("Unwrapped", wrapPtr);

const statsig = statsig_create("", unwrapPtr);

ref_release(wrapPtr);

// ref_release({
//   pointer: 0,
//   ref_type: 3,
// });
