"""CLI entry point for linux-wispr."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="wispr",
        description="linux-wispr — push-to-talk speech-to-text for Linux",
    )
    subparsers = parser.add_subparsers(dest="command")

    # ------------------------------------------------------------------ run
    run_parser = subparsers.add_parser("run", help="Start the wispr listener (default command)")
    run_parser.add_argument(
        "--config", metavar="PATH", help="Path to config.toml (default: ~/.config/linux-wispr/config.toml)"
    )
    run_parser.add_argument("--model", help="Override Whisper model size (tiny/base/small/medium/large)")
    run_parser.add_argument("--key", help="Override hotkey (e.g. right_shift, ctrl+space)")
    run_parser.add_argument("--method", help="Override output method (auto/xdotool/wtype/ydotool/clipboard)")
    run_parser.add_argument("--language", help="Override language (e.g. en, de, fr; empty=auto)")
    run_parser.add_argument("--translate", action="store_true", default=None, help="Translate speech to English")

    # -------------------------------------------------------------- init-config
    subparsers.add_parser(
        "init-config",
        help="Write a default config file to ~/.config/linux-wispr/config.toml",
    )

    # ----------------------------------------------------------- list-devices
    subparsers.add_parser("list-devices", help="List available audio input devices")

    return parser


def cmd_run(args: argparse.Namespace) -> None:
    from .config import load_config
    from .main import WisprApp

    config_path = Path(args.config) if args.config else None
    config = load_config(config_path)

    # Apply CLI overrides
    if args.model:
        config.whisper.model = args.model
    if args.key:
        config.hotkey.key = args.key
    if args.method:
        config.output.method = args.method
    if args.language is not None:
        config.whisper.language = args.language
    if args.translate:
        config.whisper.translate = True

    app = WisprApp(config=config)
    app.run()


def cmd_init_config(_args: argparse.Namespace) -> None:
    from .config import DEFAULT_CONFIG_PATH, write_default_config

    if DEFAULT_CONFIG_PATH.exists():
        print(f"Config already exists at {DEFAULT_CONFIG_PATH}")
        print("Delete it first if you want to regenerate defaults.")
        return

    path = write_default_config()
    print(f"Default config written to {path}")


def cmd_list_devices(_args: argparse.Namespace) -> None:
    from .audio import list_devices

    devices = list_devices()
    if not devices:
        print("No input devices found (or sounddevice not installed).")
        return
    print("Available audio input devices:")
    for dev in devices:
        print(f"  [{dev['index']}] {dev['name']}")


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    # Default to 'run' when no sub-command is given
    if args.command is None:
        args = parser.parse_args(["run"] + (argv or []))

    try:
        if args.command == "run":
            cmd_run(args)
        elif args.command == "init-config":
            cmd_init_config(args)
        elif args.command == "list-devices":
            cmd_list_devices(args)
        else:
            parser.print_help()
    except KeyboardInterrupt:
        pass
    except RuntimeError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1

    return 0


def entrypoint() -> None:  # noqa: D401
    """setuptools console_scripts entrypoint."""
    sys.exit(main())


if __name__ == "__main__":
    entrypoint()
