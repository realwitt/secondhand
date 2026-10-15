# Third-party notices

Secondhand includes the following third-party software. Their license texts are in `licenses/`,
and they ship inside the app.

## VectorCraft

The FreeHand reader and the `.ai`/PDF/SVG writers are VectorCraft's
(<https://github.com/storytold/vectorcraft>, crates `vectorcraft-*`).

Copyright (c) 2026 ArtCraft Team and the VectorCraft contributors. Licensed under the MIT License
or the Apache License, Version 2.0, at your option (`licenses/VectorCraft-LICENSE-MIT.txt`,
`LICENSE-APACHE`). VectorCraft's own NOTICE, including the material it bundles, is in
`licenses/VectorCraft-NOTICE.txt`. The parts of it that end up in Secondhand:

- Source Sans 3, Source Serif 4, Inter and JetBrains Mono, compiled into the text engine. SIL
  Open Font License 1.1 (`licenses/OFL-*.txt`).
- Adobe's CMap resources, compiled in through the hayro-cmap crate for reading CJK text in PDFs,
  under the BSD 3-Clause License. The full notice is in `licenses/VectorCraft-NOTICE.txt`.
- The ArtCraft name and logos are VectorCraft's trademarks and are not used by Secondhand.

The FreeHand reader was written in a clean-room way from a written description of the format (see
VectorCraft's `crates/freehand/README.md`). It contains no code from libfreehand or from Adobe.

## Tauri and Rust crates

The desktop app uses Tauri 2 (<https://tauri.app>, MIT OR Apache-2.0) and its dialog and opener
plugins (MIT OR Apache-2.0). Other Rust crates come from crates.io under their own permissive
licenses (mostly MIT and/or Apache-2.0; also BSD, ISC, Zlib, BSL-1.0 and Unicode-3.0). Each
crate's license is listed in its own `Cargo.toml` and source package.
