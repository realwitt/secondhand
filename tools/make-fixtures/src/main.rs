//! `cargo run -p make-fixtures -- fixtures/` writes the synthetic FreeHand files used by the smoke
//! tests. They are built byte by byte by VectorCraft's `synth` module: no FreeHand software and no
//! real user drawing is involved.

use vectorcraft_freehand::synth;

fn main() -> std::io::Result<()> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "fixtures".into()));
    let good = dir.join("convert");
    let bad = dir.join("refuse");
    std::fs::create_dir_all(good.join("nested folder"))?;
    std::fs::create_dir_all(&bad)?;
    let files: Vec<(std::path::PathBuf, Vec<u8>)> = vec![
        (good.join("Demo FH8.fh8"), synth::demo_at(8)),
        (good.join("Demo FH9.fh9"), synth::demo_at(9)),
        (good.join("Demo FH10.fh10"), synth::demo_at(10)),
        (good.join("Demo MX.fh11"), synth::demo_at(11)),
        (good.join("Hidden layer.fh9"), synth::hidden_layer()),
        (good.join("Content fill.fh9"), synth::content_fill()),
        (good.join("nested folder").join("Empty group and content fill.fh9"), synth::empty_group_and_content_fill()),
        (good.join("nested folder").join("Template MX.ft11"), synth::demo_at(11)),
        // An old Mac file often has no extension; it's found by its header.
        (good.join("nested folder").join("Mac file without extension"), synth::demo_at(9)),
        // Refused, each with a plain reason; the batch carries on.
        (bad.join("FreeHand 5 header.fh5"), [b"AGD1".as_slice(), &[0u8; 64]].concat()),
        (bad.join("FreeHand 7 header.fh7"), [b"AGD2".as_slice(), &[0u8; 64]].concat()),
        (bad.join("Junk.fh9"), b"This text file only pretends to be a FreeHand drawing.\n".to_vec()),
        (bad.join("Empty.fh9"), Vec::new()),
        (bad.join("Cut off.fh9"), {
            let mut d = synth::demo();
            d.truncate(d.len() / 2);
            d
        }),
    ];
    for (path, bytes) in files {
        std::fs::write(&path, bytes)?;
        println!("{}", path.display());
    }
    std::fs::write(good.join("nested folder").join("readme.txt"), "Not a drawing: ignored when the folder is converted.\n")?;
    Ok(())
}
