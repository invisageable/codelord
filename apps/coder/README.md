# apps — coder.

> *the next-gen code editor os-like for hackers*.

## about.

> *codelord doesn't care of your level, his role is to make you better day after day*.

codelord iS A PROGRAMMABLE CODE EDiTOR OS-LiKE FOR DEVS. RECLAiM THE DEVELOPER'S FLOW WiTH A HiGH-PERFORMANCE, GPU-NATiVE CODE EDiTOR THAT RESPECTS YOUR MACHiNE, YOUR PRiVACY, AND YOUR THiNKiNG PROCESS.   

codelord iS FULL NATiVE CODE EDiTOR WiTHOUT electron BLOAT AND WiTH A MiNiMAL MEMORY FOOTPRiNT. THE TELEMETRY iSN'T FOR US BUT FOR YOU. iT WiLL HELPS YOU TO BECOME MORE EFFiCiENT iN PROGRAMMiNG.

codelord ISN'T JUST ANOTHER vscode FORK OR CLONE — iT'S A REiMAGiNATiON. NATiVE GPU RENDERiNG, REPL-POWERED WORKFLOWS, REAL-TiME COLLABORATiON, AND A PLUGiN RUNTiME GiVE YOU SUPERPOWERS iN A CLEAN, MiNiMAL iNTERFACE. ALL WHiLE STAYiNG LiGHT, LOCAL-FiRST, AND PRiVACY-RESPECTiNG.

JOiN THE DEVOLUTiON.

## preview.

FOR THE MOMENT, WE ARE iNVESTiNG OUR TiME FOCUSiNG ON THE USER EXPERiENCE AND iTS iNTERFACE TO SET codelord APART FROM ALL THOSE CODE EDiTORS WHO COMPLETELY NOT ALiGNED WiTH OUR ViSiON.

> it's just a draft that's likely to evolve for the better. not for production use.

![codelord interface overview](./codelord-assets/image/preview/preview-codelord-interface-overview.gif)

> « iNTO THE TURFU WE GO. » — _compilords_.

## logging.

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

## best existing code editor.

| features      | codelord    | zed         | cursor              | vscode        |
| :------------ | :---------- | :---------- | :------------------ | :------------ |
| FOUNDATION    | rust + egui | rust + gpui | vscodium + electron | electron      |
| SiZE          | 34 MB       | 321 MB      | 458 MB              | 664 MB        |
| UX/Ui         | GAME-LiKE   | STANDARD    | STANDARD            | STANDARD      |
| Ai            | Ai PARTNER  | Ai WRAPPER  | Ai WRAPPER          | Ai COMPLETiON |
| PLAYGROUND    | ✅          | ❌          | ❌                  | ❌            |
| VOiCE CONTROL | ✅          | ❌          | ❌                  | ❌            |
| PRESENTER     | ✅          | ❌          | ❌                  | ❌            |
| EASTER EGGS   | ✅          | ❌          | ❌                  | ❌            |

## builtin features.

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
