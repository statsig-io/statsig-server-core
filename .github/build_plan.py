import json
import os
import subprocess


def partition_targets(should_build_all):
    always_build_targets = [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu"
    ]

    with open(matrix_file, 'r') as f:
        matrix_data = json.load(f)

    if should_build_all:
        include_filter = matrix_data['config']
        exclude_filter = []
    else:
        include_filter = matrix_data['config']
        exclude_filter = []
        # include_filter = [config for config in matrix_data['config'] if config['target'] in always_build_targets]
        # exclude_filter = [config for config in matrix_data['config'] if config['target'] not in always_build_targets]

    included = {'config': include_filter}
    excluded = {'config': exclude_filter}

    return included, excluded


def map_arm64_runners(included):
    if not is_private_repo:
        return included

    for config in included['config']:
        if config['runner'] == "ubuntu-24.04-arm":
            config['runner'] = "statsig-ubuntu-arm64"

    return included


def export_outputs(included):
    with open(os.environ['GITHUB_OUTPUT'], 'a') as github_output:
        github_output.write(f'build_matrix={json.dumps(included)}\n')


# -------------------------------------------------------------------- [Main]

# Load environment variables
is_merged_pr = os.getenv('IS_MERGED_PR', 'false') == 'true'
is_release_branch = os.getenv('IS_RELEASE_BRANCH', 'false') == 'true'
is_beta_branch = os.getenv('IS_BETA_BRANCH', 'false') == 'true'
is_private_repo = os.getenv('IS_PRIVATE_REPO', 'false') == 'true'
is_release_trigger = os.getenv('IS_RELEASE_TRIGGER', 'false') == 'true'
matrix_file = './.github/build_matrix.json'

should_build_all = is_release_trigger or is_release_branch or is_beta_branch
included, excluded = partition_targets(should_build_all)
included = map_arm64_runners(included)
export_outputs(included)


print(f"Is Release Branch: {is_release_branch}")
print(f"Is Beta Branch: {is_beta_branch}")

print("\n== Included ==")
print(json.dumps(included['config'], indent=2))

print("\n== Excluded ==")
print(json.dumps(excluded['config'], indent=2))
