# install.

> *One command. macOS only (for now).*

## tl;dr.

```sh
curl -fsSL https://codelord.sh/install.sh | sh
```

Then:

```sh
open /Applications/codelord.app
```

## what it does.

1. Finds the latest release on GitHub (`compilords/codelord`).
2. Picks the matching tarball (`arm64` or `x86_64`).
3. Verifies the SHA-256 checksum.
4. Drops `codelord.app` into `/Applications/`.
5. Clears the quarantine flag so macOS lets you launch it.

## options.

| Env var             | What it does                         | Default         |
|---------------------|--------------------------------------|-----------------|
| `CODELORD_VERSION`  | Pin a specific tag                   | `latest`        |
| `CODELORD_PREFIX`   | Change install directory             | `/Applications` |

Example — per-user install (no sudo):

```sh
curl -fsSL https://codelord.sh/install.sh | CODELORD_PREFIX="$HOME/Applications" sh
```

## uninstall.

```sh
rm -rf /Applications/codelord.app
```

## caveats.

- **macOS only.** Linux and Windows builds are not shipping yet.
- **Not notarized.** The installer clears `com.apple.quarantine` so
  Gatekeeper doesn't block launch. If you prefer, launch manually:
  right-click `codelord.app` → *Open* → *Open* on the warning.
- **Needs sudo** unless `CODELORD_PREFIX` points to a directory you own.
