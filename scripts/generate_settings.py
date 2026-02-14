#!/usr/bin/env python3
import argparse
from pathlib import Path

from jinja2 import Template

dir = Path(__file__).parent
template: Template = Template((dir / "settings.json.j2").read_text())
parser = argparse.ArgumentParser()
parser.add_argument("--wasm", action="store_true")
args = parser.parse_args()
(dir.parent / ".vscode" / "settings.json").write_text(template.render(wasm=args.wasm))
