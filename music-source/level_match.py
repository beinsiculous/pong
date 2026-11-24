#!/usr/bin/env python3
"""Make listening copies with equal idle RMS; preserve gameplay dynamics with one gain per score."""
from array import array
import math
from pathlib import Path
import sys
import wave

home = Path(__file__).resolve().parent / 'previews'


def read(path):
    with wave.open(str(path), 'rb') as source:
        parameters = source.getparams()
        if parameters.sampwidth != 2:
            raise ValueError('expected PCM16')
        samples = array('h', source.readframes(source.getnframes()))
        if sys.byteorder != 'little':
            samples.byteswap()
    return parameters, samples


for score in ['old', 'new']:
    _, idle = read(home/f'{score}-idle.wav')
    if not idle:
        raise ValueError(f"{score} idle render is empty; check its source assets")
    rms = math.sqrt(sum((sample / 32768) ** 2 for sample in idle) / len(idle))
    if rms == 0:
        raise ValueError(f"{score} idle render is silent; check its source assets")
    gain = 0.04 / rms
    for source in sorted(home.glob(f'{score}-*.wav')):
        if 'matched' in source.name:
            continue
        parameters, samples = read(source)
        peak = max(abs(sample * gain) for sample in samples) / 32768
        if peak >= 10 ** (-1/20):
            raise ValueError('matching would exceed -1 dBFS')
        result = array('h', (round(sample * gain) for sample in samples))
        actual_rms = math.sqrt(sum((sample / 32768) ** 2 for sample in result)/len(result))
        if sys.byteorder != 'little':
            result.byteswap()
        destination = source.with_stem(source.stem+'-matched')
        with wave.open(str(destination), 'wb') as output:
            output.setparams(parameters)
            output.writeframes(result.tobytes())
        print(f'{destination.name}: gain={gain:.6f} peak={peak:.6f} rms={actual_rms:.6f}')
