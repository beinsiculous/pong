#!/usr/bin/env python3
"""Render synthetic old/new engine auditions and matched listening copies."""
from pathlib import Path
import subprocess
import argparse
import os
import tempfile
import wave
import array
import math

home = Path(__file__).resolve().parent
game = home.parent
engine = game.parents[1] / 'insiculous_2d'
previews = home/'previews'
previews.mkdir(exist_ok=True)
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--reference', default='7165c8c8340cf25df19bed7288a17ab399b5a52a')
old_revision = parser.parse_args().reference
def reference_bytes(*arguments):
    try:
        return subprocess.check_output(['git', '-C', str(game), *arguments], stderr=subprocess.PIPE)
    except (subprocess.CalledProcessError, FileNotFoundError) as error:
        detail = error.stderr.decode(errors='replace') if isinstance(error, subprocess.CalledProcessError) else str(error)
        parser.error(f'reference {old_revision!r}: {detail.strip()}; fetch its history or choose --reference REVISION')

reference_bytes('cat-file', '-e', f'{old_revision}^{{commit}}')
banks = [os.fsdecode(path) for path in reference_bytes('ls-tree', '-rz', '--name-only', old_revision, '--', 'assets/soundfonts').split(b'\0') if path]
paths = ['assets/ai/music/ai_tong.mid', 'assets/ai/music/ai_tong.mid.ron']
paths.extend(path for path in banks if path.endswith('.sf2'))
if len(paths) == 2:
    parser.error(f'reference {old_revision!r} has no SoundFont bank')
reference = {path: reference_bytes('show', f'{old_revision}:{path}') for path in paths}
with tempfile.TemporaryDirectory(prefix='tong-reference-') as temporary:
    reference_assets = Path(temporary)
    for relative, contents in reference.items():
        destination = reference_assets/Path(relative).relative_to('assets')
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(contents)
    lines = ['# Synthetic 2P multiball bursts: 20 hits/second, with recovery gaps.\n']
    for step in range(1200):
        second = step / 10
        if step % 100 < 60:
            lines.extend([f'{second:.3f} hit 1\n', f'{second:.3f} hit 2\n'])
        if step % 3 == 0:
            lines.extend([f'{second:.3f} move 1\n', f'{second:.3f} move 2\n'])
        if step % 43 == 0:
            lines.append(f'{second:.3f} goal 3\n')
        if step % 77 == 0:
            lines.append(f'{second:.3f} menu 4\n')
    (home/'dense.tsv').write_text(''.join(lines))
    for action in ['hit', 'goal', 'move']:
        (home/f'{action}.tsv').write_text(''.join(f'{second:.3f} {action} 1\n' for second in range(1, 121)))
    subprocess.run(['cargo','build','--manifest-path',str(engine/'Cargo.toml'),'-p','music_midi','--example','audition'],check=True)
    tool = engine/'target/debug/examples/audition'
    for score, assets, recipes in [('old',reference_assets,['idle']),('new',game/'assets',['idle','sparse','dense','hit','goal','move'])]:
        for recipe in recipes:
            with (previews/f'{score}-{recipe}.log').open('w') as log:
                subprocess.run([str(tool),str(assets),'ai/music/ai_tong.mid',str(home/f'{recipe}.tsv'),'121',str(previews/f'{score}-{recipe}.wav')],stdout=log,stderr=subprocess.STDOUT,check=True)
    subprocess.run(['python3',str(home/'level_match.py')],check=True)

# Below the limiter threshold, subtracting the identical idle render isolates the
# conductor's actual action output without replacing its synthesizer or bus path.
with wave.open(str(previews/'new-idle.wav'), 'rb') as recording:
    parameters = recording.getparams()
    idle = array.array('h', recording.readframes(recording.getnframes()))
for action in ['hit', 'goal', 'move']:
    for recipe in ['idle', action]:
        if 'limited_samples=0 ' not in (previews/f'new-{recipe}.log').read_text():
            raise RuntimeError('cannot isolate a nonlinear limited mix')
    with wave.open(str(previews/f'new-{action}.wav'), 'rb') as recording:
        mixed = array.array('h', recording.readframes(recording.getnframes()))
    isolated = array.array('h', (sample - background for sample, background in zip(mixed, idle)))
    peak = max(abs(sample) for sample in isolated) / 32768
    rms = math.sqrt(sum((sample / 32768)**2 for sample in isolated) / len(isolated))
    assert 0 < rms < peak < 0.8913
    with wave.open(str(previews/f'isolated-{action}.wav'), 'wb') as recording:
        recording.setparams(parameters)
        recording.writeframes(isolated.tobytes())
    (previews/f'isolated-{action}.log').write_text(f'peak={peak:.6f} rms={rms:.6f} method=engine_mix_minus_identical_idle limiter_inactive=true\n')
