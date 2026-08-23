#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("publish.py")
SPEC = importlib.util.spec_from_file_location("d1v_publish", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
publish = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = publish
SPEC.loader.exec_module(publish)


class PublisherTests(unittest.TestCase):
    def setUp(self) -> None:
        self.publisher = publish.Publisher(Path.cwd(), dry_run=False)
        self.crate = publish.Crate("d1v-api", "0.1.8")

    def test_wait_until_published_retries_until_downloadable(self) -> None:
        with (
            patch.object(publish.Crate, "is_published", side_effect=[False, True]),
            patch.object(publish.time, "sleep") as sleep,
        ):
            self.publisher.wait_until_published(
                self.crate, timeout_seconds=1, poll_seconds=0.01
            )

        sleep.assert_called_once_with(0.01)

    def test_wait_until_published_times_out(self) -> None:
        with patch.object(publish.Crate, "is_published", return_value=False):
            with self.assertRaisesRegex(RuntimeError, "timed out waiting"):
                self.publisher.wait_until_published(
                    self.crate, timeout_seconds=0, poll_seconds=0
                )


if __name__ == "__main__":
    unittest.main()
