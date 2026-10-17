#!/usr/bin/env python3
"""Create private local-only test assets; never submit, upload or install anything."""
import argparse
from pathlib import Path
from release import write_verification_assets
p=argparse.ArgumentParser();p.add_argument('directory',type=Path);args=p.parse_args()
write_verification_assets(args.directory)
