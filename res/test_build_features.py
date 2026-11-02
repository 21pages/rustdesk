import contextlib
import importlib.util
import io
from pathlib import Path
import shlex
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "rustdesk_build", Path(__file__).resolve().parents[1] / "build.py"
)
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class MacCaptureFeaturesTests(unittest.TestCase):
    def features(self, *flags):
        with patch.object(build, "osx", True), patch.object(build, "windows", False):
            args = build.make_parser().parse_args(["--flutter", *flags])
            with contextlib.redirect_stdout(io.StringIO()):
                return build.get_features(args)

    def test_default_enables_sck_and_opt_out_keeps_resampling(self):
        self.assertIn("screencapturekit", self.features())
        self.assertIn("screencapturekit", self.features("--screencapturekit"))
        disabled = self.features("--no-screencapturekit")
        self.assertNotIn("screencapturekit", disabled)
        self.assertIn("use_dasp", disabled)

    def test_opt_out_disables_cargo_defaults_in_build_command(self):
        for flags, disabled in [((), False), (("--no-screencapturekit",), True)]:
            with self.subTest(disabled=disabled):
                features = ",".join(self.features(*flags))
                with patch.object(build, "skip_cargo", False), \
                        patch.object(build, "system2") as run, \
                        patch.object(build.os, "chdir"):
                    build.build_flutter_dmg("test", features)
                cargo = next(call[0][0] for call in run.call_args_list
                             if "cargo build" in call[0][0])
                self.assertEqual("--no-default-features" in shlex.split(cargo), disabled)
                self.assertIn(features, shlex.split(cargo))
                target = "10.14" if disabled else "12.3"
                self.assertIn("MACOSX_DEPLOYMENT_TARGET=" + target, shlex.split(cargo))
                flutter = next(call[0][0] for call in run.call_args_list
                               if "flutter build macos" in call[0][0])
                self.assertEqual("FLUTTER_XCODE_MACOSX_DEPLOYMENT_TARGET=12.3" in
                                 shlex.split(flutter), not disabled)


if __name__ == "__main__":
    unittest.main()
