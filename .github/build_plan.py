import json
import os
import subprocess

# Load environment variables
is_new_release = os.getenv('IS_NEW_RELEASE', 'false') == 'true'
is_merged_pr = os.getenv('IS_MERGED_PR', 'false') == 'true'
is_beta_branch = os.getenv('IS_BETA_BRANCH', 'false') == 'true'
is_private_repo = os.getenv('IS_PRIVATE_REPO', 'false') == 'true'
matrix_file = './.github/build_matrix.json'
should_publish = is_new_release or (is_merged_pr and is_beta_branch)


def partition_targets(should_publish):
    always_build_targets = [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu"
    ]

    with open(matrix_file, 'r') as f:
        matrix_data = json.load(f)

    if should_publish:
        include_filter = matrix_data['config']
        exclude_filter = []
    else:
        include_filter = [config for config in matrix_data['config'] if config['target'] in always_build_targets]
        exclude_filter = [config for config in matrix_data['config'] if config['target'] not in always_build_targets]

    included = {'package': matrix_data['package'], 'config': include_filter}
    excluded = {'package': matrix_data['package'], 'config': exclude_filter}

    return included, excluded


def map_arm64_runners(included):
    if not is_private_repo:
        return included

    for config in included['config']:
        if config['runner'] == "ubuntu-24.04-arm":
            config['runner'] = "statsig-ubuntu-arm64"

    return included


def export_outputs(included, should_publish):
    with open(os.environ['GITHUB_OUTPUT'], 'a') as github_output:
        github_output.write(f'build_matrix={json.dumps(included)}\n')
        github_output.write(f'should_publish={should_publish}\n')

included, excluded = partition_targets(should_publish)
included = map_arm64_runners(included)
export_outputs(included, should_publish)


print(f"Is Release Branch: {os.getenv('IS_RELEASE_BRANCH', 'false')}")
print(f"Is Beta Branch: {is_beta_branch}")
print(f"Should Publish: {should_publish}")

print("\n== Included ==")
print(json.dumps(included['config'], indent=2))

print("\n== Excluded ==")
print(json.dumps(excluded['config'], indent=2))
