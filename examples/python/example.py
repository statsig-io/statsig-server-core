import zipfile
import os
import sys
import pprint

whl_file_path = (
    "../../target/wheels/statsig_python_core-0.1.0-cp37-abi3-macosx_11_0_arm64.whl"
)
extract_dir = "statsig_python_core_extracted"

with zipfile.ZipFile(whl_file_path, "r") as zip_ref:
    zip_ref.extractall(extract_dir)

sys.path.append(os.path.abspath(extract_dir))

import statsig_python_core

pprint.pprint(
    [
        (cls, getattr(statsig_python_core, cls).__dict__)
        for cls in dir(statsig_python_core)
        if isinstance(getattr(statsig_python_core, cls), type)
    ],
    indent=4,
)

key = os.getenv("STATSIG_SECRET_KEY")
statsig = statsig_python_core.Statsig(key)

result = statsig.initialize()

print("initialize result", result)

user = statsig_python_core.StatsigUser("a-user")
gate_result = statsig.check_gate("a_gate", user)

print("check_gate_result", gate_result)

gate_result = statsig.get_feature_gate("a_gate", user)
print(
    "Class of gate_result:",
    gate_result.name,
    gate_result.value,
    gate_result.id_type,
    gate_result.rule_id,
)

exp_result = statsig.get_experiment("another_experiment", user)
print("exp_result:", exp_result.name, exp_result.group_name)
print("exp_result str:", exp_result.get_string("a_string"))
print("exp_result bool:", exp_result.get_bool("a_bool"))
print("exp_result number:", exp_result.get_number("a_number"))


gcir = statsig.get_client_init_response(user)
print("gcir:", gcir[:300])
