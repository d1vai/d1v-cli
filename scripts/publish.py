#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CRATES = ["d1v-api", "d1v-cli"]
PUBLISH_TIMEOUT_SECONDS = 300
PUBLISH_POLL_SECONDS = 5


@dataclass(frozen=True)
class Crate:
    name: str
    version: str

    @property
    def download_url(self) -> str:
        return f"https://static.crates.io/crates/{self.name}/{self.version}/download"

    def is_published(self) -> bool:
        try:
            with urlopen(Request(self.download_url, method="HEAD")):
                return True
        except HTTPError as error:
            match error.code:
                case 403 | 404:
                    return False
                case _:
                    raise RuntimeError(
                        f"failed to check {self.name} {self.version} at {self.download_url}"
                    ) from error


class Publisher:
    def __init__(self, root: Path, dry_run: bool) -> None:
        self.root = root
        self.dry_run = dry_run

    def resolve(self, names: list[str]) -> list[Crate]:
        output = subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=self.root,
            text=True,
        )
        crates = {
            pkg["name"]: Crate(name=pkg["name"], version=pkg["version"])
            for pkg in json.loads(output)["packages"]
        }

        try:
            return [crates[name] for name in names]
        except KeyError as error:
            raise RuntimeError(f"package not found in cargo metadata: {error.args[0]}") from error

    def publish(self, crate: Crate) -> None:
        command = ["cargo", "publish", "-p", crate.name, "--locked"]
        if self.dry_run:
            command.append("--dry-run")

        print(f"Running: {' '.join(command)}")
        subprocess.check_call(command, cwd=self.root)

    def wait_until_published(
        self,
        crate: Crate,
        *,
        timeout_seconds: float = PUBLISH_TIMEOUT_SECONDS,
        poll_seconds: float = PUBLISH_POLL_SECONDS,
    ) -> None:
        deadline = time.monotonic() + timeout_seconds
        while not crate.is_published():
            if time.monotonic() >= deadline:
                raise RuntimeError(
                    f"timed out waiting for {crate.name} {crate.version} on crates.io"
                )
            print(
                f"Waiting for {crate.name} {crate.version} to become downloadable...",
                flush=True,
            )
            time.sleep(poll_seconds)

        print(f"Available on crates.io: {crate.name} {crate.version}.")

    def run(self, names: list[str]) -> None:
        if self.dry_run:
            print("Dry run: cargo publish will be invoked with --dry-run.")

        published = 0
        for crate in self.resolve(names):
            if crate.is_published():
                print(f"Skip {crate.name} {crate.version}: already on crates.io.")
                continue
            self.publish(crate)
            if not self.dry_run:
                self.wait_until_published(crate)
            published += 1

        if published:
            print(f"Done. Published {published} crate(s).")
        else:
            print("Nothing to do; all versions already published.")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Publish workspace crates to crates.io, skipping versions already there.",
    )
    parser.add_argument(
        "crates",
        nargs="*",
        default=DEFAULT_CRATES,
        metavar="CRATE",
        help=(
            "Crate names to publish in dependency order "
            f"(default: {', '.join(DEFAULT_CRATES)})."
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Pass --dry-run to cargo publish.",
    )

    return parser.parse_args()


def main() -> int:
    args = parse_args()

    try:
        Publisher(root=REPO_ROOT, dry_run=args.dry_run).run(args.crates)
    except RuntimeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as error:
        return error.returncode

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
