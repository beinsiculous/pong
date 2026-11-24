//! Install the movement map through the Conductor editor's validated edit/save API.
use std::{error::Error, path::Path};
use editor::song::{Gesture, SongDocument, SongEdit};
use music_midi::{Binding, Instrument, InstrumentSource, Part, PartMode, Quantize, TrackSelector};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() > 1 { return Err("usage: install_movement [ASSETS]".into()); }
    let assets = arguments.first().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"), std::path::PathBuf::from);

    let path = assets.join("ai/music/ai_tong.mid");
    let mut document = SongDocument::open(&path, &assets)?.document;
    let file = document.file();
    let part_exists = file.parts.iter().any(|part| part.name == "movement");
    let binding = file.bindings.iter().find(|binding| binding.action == "move");
    let complete = part_exists && binding.is_some_and(|binding| {
        binding.part == "movement" && file.instruments.iter().any(|instrument| instrument.name == binding.instrument)
    });
    if !complete && (part_exists || binding.is_some() || file.instruments.iter().any(|instrument| instrument.name == "movement")) {
        return Err("partial movement map; repair its instrument and binding in the Conductor panel".into());
    }
    if !complete {
        document.apply(SongEdit::Batch(vec![
            SongEdit::AddPart { part: Part { name: "movement".into(), track: TrackSelector::Name("movement".into()), mode: PartMode::Loop, quantize: Quantize::Off } },
            SongEdit::AddInstrument { instrument: Instrument { name: "movement".into(), provenance: None, source: InstrumentSource::SoundFont { path: "soundfonts/ai_tong.sf2".into(), bank: 0, preset: 57 } } },
            SongEdit::SetBinding { binding: Binding { action: "move".into(), part: "movement".into(), instrument: "movement".into() } },
        ]), Gesture::Discrete)?;
    }
    document.save(&path, &assets)?;
    println!("saved {} through SongDocument", path.display());
    Ok(())
}
