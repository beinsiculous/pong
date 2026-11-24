#!/usr/bin/env python3
"""Author Countertop Circuit: an original 32-bar D-minor electro-funk score."""
import json
from pathlib import Path

SOURCE_DIRECTORY = Path(__file__).resolve().parent


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def note(pitch, start, length=180, velocity=100):
    return dict(pitch=pitch, start_tick=start, duration_ticks=length, velocity=velocity)


def main():
    specifications = [
        ('rubber_bass', 0, 'saw', 0.34, 0.18, True),
        ('neon_lead', 1, 'square', 0.20, 0.24, True),
        ('glass_chord', 2, 'triangle', 0.30, 0.32, True),
        ('contact', 56, 'sine', 0.68, 0.075, True),
        ('goal', 47, 'triangle', 1.0, 0.14, True),
        ('kick', 0, 'sine', 0.85, 0.16, False),
        ('snare', 0, 'noise', 0.40, 0.10, False),
        ('hat', 0, 'noise', 0.16, 0.025, False),
        ('movement', 57, 'triangle', 0.16, 0.035, True),
    ]
    instruments = []
    for index, (name, program, waveform, gain, decay, pitched) in enumerate(specifications):
        oscillator = dict(waveform=waveform, start_frequency_hz=440.0 if pitched else (150.0 if name=="kick" else 58.0), end_frequency_hz=440.0 if pitched else 48.0, amplitude=1.0)
        patch = dict(name=name, sample_rate=32000, seed=4100 + index, gain=gain,
                     duration_seconds=0.8, pitch_tracking=pitched,
                     oscillators=[oscillator], envelope=dict(attack_seconds=0.002,
                     decay_seconds=decay, sustain_level=0.0, release_seconds=0.04))
        if name == 'rubber_bass':
            patch['filter'] = dict(cutoff_hz=700.0, envelope_amount=2.5, resonance=0.25, stages=2)
        if name == 'hat':
            patch['filter'] = dict(mode='high_pass', cutoff_hz=6500.0, stages=2)
        if name == 'snare':
            patch['filter'] = dict(mode='high_pass', cutoff_hz=900.0, stages=1)
        if name in ('contact', 'glass_chord'):
            patch['oscillators'].append(dict(waveform='sine', start_frequency_hz=1760.0,
                                            end_frequency_hz=1760.0, amplitude=0.35))
        if name == 'goal':
            oscillator['start_frequency_hz'] = 110.0
            oscillator['end_frequency_hz'] = 110.0
        save(SOURCE_DIRECTORY / 'patches' / f'{name}.json', patch)
        instruments.append(dict(name=name, source=dict(type='synth', patch=f'patches/{name}.json'),
                                midi_program=program, gain=1.0, pan=0.0))
    tracks = [dict(name=name, instrument=index, notes=[]) for name, index in
              [('Bass', 0), ('Lead', 1), ('Chords', 2), ('Kick', 5), ('Snare', 6), ('Hat', 7),
               ('melody', 3), ('jam', 3), ('movement', 8)]]
    # Each four-bar sentence has a distinct rhythm and a returning D-A-F signature.
    hooks = [
        [(0,74),(360,77),(720,81),(1080,77),(1440,74),(1680,81)],
        [(120,74),(480,72),(840,69),(1320,72),(1560,74)],
        [(0,77),(240,81),(600,84),(960,81),(1320,77),(1680,74)],
        [(0,72),(360,69),(720,74),(1200,77),(1560,74)],
        [(0,86),(360,81),(600,77),(1080,81),(1440,84)],
        [(120,81),(480,77),(840,74),(1200,72),(1680,74)],
        [(0,77),(480,74),(720,69),(1080,72),(1440,77)],
        [(0,81),(240,77),(720,74),(1080,72),(1440,69),(1680,74)],
    ]
    for bar in range(32):
        start = bar * 1920
        breakdown = 16 <= bar < 24
        for offset, pitch in [(0,38),(360,38),(720,45),(960,38),(1320,50),(1680,45)]:
            if not breakdown or offset in (0,720,1320):
                tracks[0]['notes'].append(note(pitch,start+offset,150,112 if offset==0 else 92))
        hook = hooks[bar % 4 if bar < 8 or bar >= 24 else 4 + bar % 4]
        if not breakdown or bar % 2 == 1:
            for position, (offset, pitch) in enumerate(hook):
                tracks[1]['notes'].append(note(pitch,start+offset,180 if position%2==0 else 120,100 if bar>=24 else 88))
        for offset in ([240,1200] if breakdown else [240,840,1200,1800]):
            for pitch in ([62,65,69] if bar%4<2 else [62,69,72]):
                tracks[2]['notes'].append(note(pitch,start+offset,120,65 if breakdown else 80))
        for offset in ([0,960] if breakdown else [0,720,960,1560]):
            tracks[3]['notes'].append(note(36,start+offset,120,118 if offset%960==0 else 92))
        for offset in [480,1440]:
            tracks[4]['notes'].append(note(38,start+offset,100,112))
        for step in range(8):
            if not breakdown or step%2:
                tracks[5]['notes'].append(note(42,start+step*240,60,76 if step%2 else 52))
        if bar%8==7:
            for offset in [1500,1620,1740,1860]:
                tracks[4]['notes'].append(note(38,start+offset,50,65+(offset-1500)//12))
    # The ending is a real short note, not a timing-only event.
    tracks[0]['notes'].append(note(38,61320,120,110))
    for index, pitch in enumerate([74,81,77,74,69,72,74,77]):
        tracks[6]['notes'].append(note(pitch,index*240,70,95))
    for index, pitch in enumerate([74,77,81,77]):
        tracks[7]['notes'].append(note(pitch,index*240,65,80))
    for index, pitch in enumerate([74,77,81,72]):
        tracks[8]['notes'].append(note(pitch,index*240,40,72))
    project = dict(format='midi-evil-project',version=2,name='Countertop Circuit',tempo_bpm=128,
                   beats_per_bar=4,beat_unit=4,ticks_per_quarter=480,sample_rate=32000,
                   master_gain=0.8,instruments=instruments,tracks=tracks)
    (SOURCE_DIRECTORY/'countertop.evil').write_text(json.dumps(project, separators=(',', ':')) + '\n')
    presets = []
    renders = []
    for index, (name, program, _, _, _, pitched) in enumerate(specifications):
        keys = sorted({entry['pitch'] for track in tracks if track['instrument']==index for entry in track['notes']})
        if index==4:
            keys = sorted({entry['pitch'] for entry in tracks[6]['notes']})
        zones = []
        for key in keys:
            filename=f'rendered/{name}_{key}.wav'
            renders.append(dict(instrument=index,key=key,ticks=480,wav=filename))
            zones.append(dict(wav=filename,keys=[key,key],root=key,release_seconds=0.04,
                              longest_note_seconds=0.24))
        if pitched:
            presets.append(dict(name=name,bank=0,program=program,zones=zones))
        else:
            percussion=next((preset for preset in presets if preset['bank']==128),None)
            if percussion is None:
                percussion=dict(name='Circuit drums',bank=128,program=0,zones=[])
                presets.append(percussion)
            percussion['zones'].extend(zones)
    save(SOURCE_DIRECTORY/'bank.json',dict(version=1,name='Countertop Circuit',copyright='Original AI synthesis; Be Insiculous; CC0-1.0',presets=presets))
    save(SOURCE_DIRECTORY/'render.json',dict(seed=4100,sample_rate=32000,velocity=127,gain=0.8,
                              backing_pitch_classes=[0,2,5,9],cue_pitch_classes=[0,2,5,9],renders=renders))


if __name__=='__main__':
    main()
