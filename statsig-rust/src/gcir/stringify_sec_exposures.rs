use std::collections::HashMap;

use crate::{hashing::HashUtil, HashAlgorithm, SecondaryExposure};

pub(crate) fn stringify_sec_exposures(
    secondary_exposures: &Vec<SecondaryExposure>,
    hashing: &HashUtil,
    resulting_exposures: &mut HashMap<String, SecondaryExposure>,
) {
    for exposure in secondary_exposures {
        let key = format!(
            "{}:{}:{}",
            exposure.gate,
            exposure.gate_value,
            exposure.rule_id.as_str()
        );
        let hash = hashing.hash(&key, &HashAlgorithm::Djb2);

        resulting_exposures.insert(hash, exposure.clone());
    }
}
