import zipfile
import os
import sys
import pprint

whl_file_path = (
    "../../target/wheels/sigstat_python_core-0.1.0-cp37-abi3-macosx_11_0_arm64.whl"
)
extract_dir = "sigstat_python_core_extracted"

with zipfile.ZipFile(whl_file_path, "r") as zip_ref:
    zip_ref.extractall(extract_dir)

sys.path.append(os.path.abspath(extract_dir))

import sigstat_python_core

pprint.pprint(
    [
        (cls, getattr(sigstat_python_core, cls).__dict__)
        for cls in dir(sigstat_python_core)
        if isinstance(getattr(sigstat_python_core, cls), type)
    ],
    indent=4,
)

key = os.getenv("STATSIG_SECRET_KEY")
statsig = sigstat_python_core.StatsigPy(key)

result = statsig.initialize()

print("initialize result", result)

user = sigstat_python_core.StatsigUserPy("a-user")
gate_result = statsig.check_gate("a_gate", user)

print("gate_result", gate_result)
