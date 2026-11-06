import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class BuildPlanTests(unittest.TestCase):
    def plan(self, **flags):
        env = os.environ.copy()
        for flag in (
            "IS_MERGED_PR", "IS_RELEASE_BRANCH", "IS_BETA_BRANCH",
            "IS_PRIVATE_REPO", "IS_RELEASE_TRIGGER", "IS_MAIN_BRANCH",
            "FORCE_FULL_BUILD",
        ):
            env[flag] = "true" if flags.get(flag, False) else "false"
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            env["GITHUB_OUTPUT"] = str(output)
            subprocess.run(
                [sys.executable, ".github/build_plan.py"],
                cwd=ROOT, env=env, check=True, capture_output=True, text=True,
            )
            return json.loads(output.read_text().split("=", 1)[1])["config"]

    def test_normal_pr_uses_small_matrix(self):
        configs = self.plan(IS_PRIVATE_REPO=True)
        self.assertTrue(configs)
        self.assertTrue(all(config.get("always_build") for config in configs))

    def test_manual_private_preflight_covers_mac_arm64(self):
        configs = self.plan(IS_PRIVATE_REPO=True, FORCE_FULL_BUILD=True)
        arm = next(c for c in configs if c["target"] == "aarch64-apple-darwin")
        self.assertEqual(arm["runner"], "macos-15")
        self.assertTrue(any(c["runner"] == "statsig-ubuntu-arm64" for c in configs))
        self.assertFalse(any(c["runner"] == "windows-11-arm" for c in configs))

    def test_public_release_retains_complete_target_set(self):
        configs = self.plan(IS_RELEASE_TRIGGER=True)
        targets = {c["target"] for c in configs}
        expected = {c["target"] for c in json.loads(
            (ROOT / ".github/build_matrix_v2.json").read_text()
        )["config"]}
        self.assertEqual(targets, expected)
        self.assertFalse(any(c["runner"] == "macos-14" for c in configs))


if __name__ == "__main__":
    unittest.main()
