import os
from statsig_python_core import Statsig, StatsigOptions, StatsigUser

# Initialize Statsig
key = os.getenv("STATSIG_SECRET_KEY")
if key is None:
    raise ValueError("STATSIG_SECRET_KEY is not set in environment variables.")

# Customized statsigOptions as needed
options = StatsigOptions()
options.environment = "development"
options.output_log_level = "error"

statsig = Statsig(key, options)
statsig.initialize().wait()

# Create a statsig user，passing fields as needed
user = StatsigUser(
    user_id="user_123",
    email="user@gmail.com",
    country="US",
    # Note: The following fields must be Python dictionaries (PyDict) due to PyO3 bindings
    # - `custom`: A dictionary where all keys must be strings, and values can be JSON-compatible types
    #   (e.g., str, int, bool, list, dict). Example: {"key1": "value1", "key2": 123}
    custom={},
    # - `custom_ids`: A dictionary where both keys and values must be strings. Example: {"companyID": "12345"}
    custom_ids={},
    # - `private_attributes`: A dictionary where all keys must be strings, and values can be JSON-compatible types.
    #   This is used for storing user-specific private metadata. Example: {"attr1": "data"}
    private_attributes={},
)

# Check a feature gate
gate_result = statsig.check_gate("a_gate", user)
print("Feature Gate Result:", gate_result)

# Retrieve experiment details
exp_result = statsig.get_experiment("another_experiment", user)
print("Experiment Details:")
print(" - Group Name:", exp_result.group_name)
print(" - Id Type:", exp_result.id_type)

# Get a dynamic Config
config = statsig.get_dynamic_config(user, "config_name")

# Get a Layer
layer = statsig.get_layer(user, "layer_name")
print("Layer Details:")
print(" - allocatedExperiment:", layer.allocated_experiment_name)
print(" - group name:", layer.group_name)
print(" - id type:", layer.id_type)

# Shutting down statsig
statsig.shutdown().wait()

print("Sample Statsig app executed successfully!")
