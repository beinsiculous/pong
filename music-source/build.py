#!/usr/bin/env python3
"""Stage Countertop Circuit with the shared MIDI Evil build pipeline."""
from pathlib import Path
import subprocess
import sys

source = Path(__file__).resolve().parent
subprocess.run(['python3', str(source.parents[2]/'insiculous_2d/scripts/build_music.py'),
                str(source/'countertop.evil'), 'ai_tong', *sys.argv[1:]], check=True)
