# apps — coder.

> *Understanding the whole codelord's architecture.*

## dev.

### logging.

WE USE THE RUST `log` CRATE FOR STRUCTURED LOGGiNG. CONTROL LOG OUTPUT USiNG THE `RUST_LOG` ENViRONMENT VARiABLE:

```bash
# show only errors.
RUST_LOG=error cargo run --package codelord
# show errors and warnings.
RUST_LOG=warn cargo run --package codelord
# show errors, warnings, and info messages.
RUST_LOG=info cargo run --package codelord
# show all messages including debug (verbose).
RUST_LOG=debug cargo run --package codelord
# enable debug logging for specific modules.
RUST_LOG=codelord_coder=debug cargo run --package codelord
# mix different levels for different modules.
RUST_LOG=codelord_coder=debug,codelord_data=info cargo run --package codelord
```

### builtin features.

**viewer**

- FONT ViEWER — `.ttf`, `otf`.
- PDF ViEWER — `pagination`, `zoom`.
- SQLiTE ViEWER — `.db`, `.sqlite`, `.sqlite3`.
- SVG ViEWER — `zoom`.
- HTML ViEWER — `hot-reloading`.
- CSV ViEWER.
- XLS ViEWER.
- AUDiO ViEWER — `.mp3`, `.wav`.

**syntax higlighting**

codelord SUPPORTS ALL THESE TYPE OF FiLES BY DEFAULT:

`bash`, `c`, `css`, `conf`, `csv`, `elixir`, `gleam`, `go`, `html`, `javascript`, `json`, `markdown`, `ocaml`, `python`, `toml`, `typescript`, `yaml`, `zig`, `zo`.

**symbol track scrollbar**

THE SYMBOL TRACK SCROLLBAR iS AN iNNOVATiVE COMPONENT iN EDiTOR ViEW TO REPLACE THE MiNiMAP. iT PROViDES THE FULL SYMBOLS iN THE FiLE LiKE `items` and `statements`.

**filescope**

iNSPIRED BY ViM'S TELESCOPE, FiLESCOPE iT'S A FASTEST FiLE FINDER THAT'S LET YOU SEARCH ANY FiLES iN YOUR WORKSPACE.

## contributing.

WE LOVE CONTRiBUTORS.   

FEEL FREE TO OPEN AN iSSUE iF YOU WANT TO CONTRiBUTE OR COME TO SAY HELLO ON [discord](https://discord.gg/JaNc4Nk5xw). ALSO YOU CAN CONTACT US AT THE [at] COMPiLORDS [dot] HOUSE. THiS iS A PLAYGROUND FOR COMPiLER __NERDS__, FRONTEND __HACKERS__, AND __CREATIVE__.    

## license.

[apache](./LICENSE-APACHE) — [mit](./LICENSE-MIT)

COPYRiGHT© **29** JULY **2024** — *PRESENT, [@invisageable](https://twitter.com/invisageable) — [@compilords](https://twitter.com/compilords) team.*
