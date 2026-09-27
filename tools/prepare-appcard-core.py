#!/usr/bin/env python3
"""Prepare AppCard's private Octos profile from an existing model configuration."""

import argparse
import fcntl
import json
import os
from pathlib import Path
import tempfile


def write_private(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as output:
            json.dump(value, output, ensure_ascii=False, indent=2)
            output.write("\n")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-home", required=True, type=Path)
    parser.add_argument("--profile", default="octos")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if not args.profile or Path(args.profile).name != args.profile or args.profile in (".", ".."):
        parser.error("profile must be one profile id")
    source_home = args.source_home.expanduser().resolve()
    output = args.output.expanduser().resolve()
    if output == source_home or source_home in output.parents or output in source_home.parents:
        parser.error("AppCard must use a separate data directory outside the source Octos home")
    source = source_home / "profiles" / (args.profile + ".json")
    try:
        profile = json.loads(source.read_text())
        config = profile["config"]
        primary = config["llm"]["primary"]
        if not profile.get("enabled", True) or not primary:
            parser.error("source profile must be enabled and have a primary model")
    except (OSError, ValueError, KeyError):
        parser.error("cannot read a configured Octos profile at " + str(source))
    output.mkdir(parents=True, exist_ok=True, mode=0o700)
    lock = (output / ".octos-serve.lock").open("a+b")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        parser.error("AppCard's Octos core is already running; close that AppCard session before restarting dev")
    # Reuse model selection and credentials, without inheriting inner-agent
    # plugins, hooks, permission settings or runtime database paths.
    model_config = {key: config[key] for key in ("llm", "env_vars", "api_type") if key in config}
    gateway = config.get("gateway") or {}
    inference = {key: gateway[key] for key in ("llm_temperature", "reasoning_effort", "llm_sampling_params") if key in gateway}
    if inference:
        model_config["gateway"] = inference
    derived = {key: profile[key] for key in ("created_at", "updated_at") if key in profile}
    derived.update(id="_main", name="AppCard", enabled=True,
                   data_dir=str(output / "profiles" / "_main" / "data"), config=model_config)
    write_private(output / "profiles" / "_main.json", derived)
    write_private(output / "config.json", {"mode": "local"})
    # Catalog routes and auth records can be required by a selected provider.
    # Keep copies private; the source registry and runtime are never opened for writes.
    for filename in ("model_catalog.json", "auth.json"):
        path = source_home / filename
        if path.is_file():
            write_private(output / filename, json.loads(path.read_text()))
    print("AppCard: prepared _main from " + args.profile + " in " + str(output))


if __name__ == "__main__":
    main()
