#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SecondaryExposureKey {
    pub gate_name_hash: u64,
    pub rule_id_hash: u64,
    pub gate_value_hash: u64,
}

impl SecondaryExposureKey {
    pub fn new(gate_name_hash: u64, rule_id_hash: u64, gate_value_hash: u64) -> Self {
        Self {
            gate_name_hash,
            rule_id_hash,
            gate_value_hash,
        }
    }
}
