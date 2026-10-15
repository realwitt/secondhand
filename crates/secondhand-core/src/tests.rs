use super::*;
use vectorcraft_freehand::synth;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("secondhand-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn freehand_extensions() {
    for ok in ["a.fh", "a.FH9", "a.fh11", "a.ft8", "a.Ft11"] {
        assert!(has_freehand_extension(Path::new(ok)), "{ok}");
    }
    for no in ["a.fhx", "a.ai", "a.f", "a", "a.fh123"] {
        assert!(!has_freehand_extension(Path::new(no)), "{no}");
    }
}

#[test]
fn a_folder_batch_converts_skips_and_fails_per_file_and_never_overwrites() {
    let d = tmp("batch");
    let sub = d.join("old work").join("1999");
    fs::create_dir_all(&sub).unwrap();
    fs::write(d.join("Logo.fh9"), synth::demo()).unwrap();
    fs::write(d.join("Logo.ai"), b"already here").unwrap();
    fs::write(sub.join("Poster.fh8"), synth::demo_at(8)).unwrap();
    fs::write(sub.join("NoExtension"), synth::demo_at(11)).unwrap();
    fs::write(sub.join("photo.jpg"), b"\xff\xd8\xff").unwrap();
    fs::write(sub.join("notes"), b"just some text").unwrap();
    fs::write(d.join("Ancient.fh5"), [b"AGD1".as_slice(), &[0u8; 64]].concat()).unwrap();
    fs::write(d.join("Junk.fh10"), b"this is not a drawing at all").unwrap();
    let mut cut = synth::demo();
    cut.truncate(cut.len() / 2);
    fs::write(d.join("Cut.fh9"), &cut).unwrap();
    let before = fs::read(d.join("Logo.fh9")).unwrap();

    let mut events = 0;
    let s = run(&[d.clone()], Format::Ai, |_| events += 1);
    let report = s.report();
    println!("{report}");
    assert_eq!(s.converted.len(), 3, "{report}");
    assert_eq!(s.skipped.len(), 2, "{report}");
    assert_eq!(s.failed.len(), 1, "{report}");
    assert_eq!(s.ignored_in_folders, 3, "{report}");
    assert_eq!(events, 1 + 2 * 6);
    assert_eq!(fs::read(d.join("Logo.ai")).unwrap(), b"already here");
    let new = fs::read(d.join("Logo (1).ai")).unwrap();
    assert!(new.starts_with(b"%PDF-"), "the .ai is a PDF-compatible file");
    assert!(sub.join("Poster.ai").exists() && sub.join("NoExtension.ai").exists());
    assert_eq!(fs::read(d.join("Logo.fh9")).unwrap(), before, "originals are never modified");
    assert!(report.contains("FreeHand 4 or 5"), "{report}");
    assert!(report.contains("isn't a FreeHand file"), "{report}");
    assert!(report.contains("incomplete"), "{report}");
    let _ = fs::remove_dir_all(d);
}

#[test]
fn pdf_and_svg_outputs() {
    let d = tmp("formats");
    let f = d.join("Hidden.fh9");
    fs::write(&f, synth::hidden_layer()).unwrap();
    let Outcome::Converted { outputs, .. } = convert(&f, Format::Pdf) else { panic!() };
    assert!(fs::read(&outputs[0]).unwrap().starts_with(b"%PDF-"));
    let Outcome::Converted { outputs, .. } = convert(&f, Format::Svg) else { panic!() };
    assert!(fs::read_to_string(&outputs[0]).unwrap().contains("<svg"));
    let _ = fs::remove_dir_all(d);
}

#[test]
fn hostile_files_fail_without_stopping_anything() {
    let d = tmp("hostile");
    fs::write(d.join("deep.fh9"), synth::nested_groups(10_000)).unwrap();
    fs::write(d.join("cycle.fh9"), synth::cyclic_groups()).unwrap();
    fs::write(d.join("symbol.fh9"), synth::cyclic_symbol()).unwrap();
    fs::write(d.join("empty.fh9"), b"").unwrap();
    let s = run(&[d.clone()], Format::Ai, |_| {});
    assert_eq!(s.converted.len() + s.skipped.len() + s.failed.len(), 4, "{}", s.report());
    let _ = fs::remove_dir_all(d);
}

#[test]
fn a_missing_path_is_reported() {
    let s = run(&[PathBuf::from("/definitely/not/here.fh9")], Format::Ai, |_| {});
    assert_eq!(s.failed.len(), 1);
}

#[test]
fn names_are_safe() {
    assert_eq!(sanitize("a/b:c?"), "a_b_c_");
    assert_eq!(sanitize("  ..."), "Untitled");
}
