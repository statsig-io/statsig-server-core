use crate::{
    networking::ResponseData,
    observability::ops_stats::OpsStatsForInstance,
    specs_response::{proto_specs::deserialize_protobuf, spec_types::SpecsResponseFull},
};

const EVAL_PROJ_PROTO_BYTES: &[u8] = include_bytes!("../../../tests/data/eval_proj_dcs.pb.br");
const DEMO_PROJ_PROTO_BYTES: &[u8] = include_bytes!("../../../tests/data/demo_proj_dcs.pb.br");

const EVAL_PROJ_CHECKSUM: &str = "9073779682072068000" /* eval_proj_dcs.json['checksum'] */;
const EVAL_PROJ_GATE_COUNT: usize = 82 /* eval_proj_dcs.json['feature_gates'].length */;
const EVAL_PROJ_DC_COUNT: usize = 65 /* eval_proj_dcs.json['dynamic_configs'].length */;
const EVAL_PROJ_LAYER_COUNT: usize = 12 /* eval_proj_dcs.json['layer_configs'].length */;

const DEMO_PROJ_CHECKSUM: &str = "1866813505520271400" /* demo_proj_dcs.json['checksum'] */;
const DEMO_PROJ_GATE_COUNT: usize = 9 /* demo_proj_dcs.json['feature_gates'].length */;
const DEMO_PROJ_DC_COUNT: usize = 7 /* demo_proj_dcs.json['dynamic_configs'].length */;
const DEMO_PROJ_LAYER_COUNT: usize = 1 /* demo_proj_dcs.json['layer_configs'].length */;

lazy_static::lazy_static! {
    static ref OPS_STATS: OpsStatsForInstance = OpsStatsForInstance::new();
}

#[test]
fn test_deserialize_eval_proj_proto() {
    let left = SpecsResponseFull::default();
    let mut right = SpecsResponseFull::default();

    let mut data = ResponseData::from_bytes(EVAL_PROJ_PROTO_BYTES.to_vec());

    deserialize_protobuf(&OPS_STATS, &left, &mut right, &mut data).unwrap();

    assert_eq!(right.checksum.as_deref(), Some(EVAL_PROJ_CHECKSUM));
    assert_eq!(right.feature_gates.len(), EVAL_PROJ_GATE_COUNT);
    assert_eq!(right.dynamic_configs.len(), EVAL_PROJ_DC_COUNT);
    assert_eq!(right.layer_configs.len(), EVAL_PROJ_LAYER_COUNT);
}

#[test]
fn test_deserialize_demo_proj_proto() {
    let left = SpecsResponseFull::default();
    let mut right = SpecsResponseFull::default();

    let mut data = ResponseData::from_bytes(DEMO_PROJ_PROTO_BYTES.to_vec());

    deserialize_protobuf(&OPS_STATS, &left, &mut right, &mut data).unwrap();

    assert_eq!(right.checksum.as_deref(), Some(DEMO_PROJ_CHECKSUM));
    assert_eq!(right.feature_gates.len(), DEMO_PROJ_GATE_COUNT);
    assert_eq!(right.dynamic_configs.len(), DEMO_PROJ_DC_COUNT);
    assert_eq!(right.layer_configs.len(), DEMO_PROJ_LAYER_COUNT);
}

#[test]
fn test_continuous_swapping() {
    let mut curr = SpecsResponseFull::default();
    let mut next = SpecsResponseFull::default();

    for i in 0..10 {
        if i % 4 == 0 {
            let mut data = ResponseData::from_bytes(EVAL_PROJ_PROTO_BYTES.to_vec());
            deserialize_protobuf(&OPS_STATS, &curr, &mut next, &mut data).unwrap();
        } else {
            let mut data = ResponseData::from_bytes(DEMO_PROJ_PROTO_BYTES.to_vec());
            deserialize_protobuf(&OPS_STATS, &curr, &mut next, &mut data).unwrap();
        }

        std::mem::swap(&mut curr, &mut next);
        next.reset();
    }

    assert_eq!(curr.checksum.as_deref(), Some(DEMO_PROJ_CHECKSUM));
    assert_eq!(curr.feature_gates.len(), DEMO_PROJ_GATE_COUNT);
    assert_eq!(curr.dynamic_configs.len(), DEMO_PROJ_DC_COUNT);
    assert_eq!(curr.layer_configs.len(), DEMO_PROJ_LAYER_COUNT);

    assert_eq!(next.checksum.as_deref(), None);
}
