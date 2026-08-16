<div align="center">

<img src="src-tauri/icons/128x128.png" width="88" alt="readingcomics">

# readingcomics

**A fast, native comic reader for your desktop — that also tells you which issues you're missing.**

[![Release](https://img.shields.io/github/v/release/DarioRQ/readingcomics?display_name=tag&sort=semver)](https://github.com/DarioRQ/readingcomics/releases/latest)
[![License](https://img.shields.io/github/license/DarioRQ/readingcomics)](./LICENSE)
![Windows](https://img.shields.io/badge/Windows-supported-blue)
![Linux](https://img.shields.io/badge/Linux-supported-blue)

[Download](https://github.com/DarioRQ/readingcomics/releases/latest) ·
[Report a bug](https://github.com/DarioRQ/readingcomics/issues) ·
[Español](./README.es.md)

</div>

---

## Why another comic reader?

Most desktop CBZ/CBR readers either haven't been touched in a decade, or they're
a server you have to host and open in a browser. This one is a **normal desktop
app**: you install it, point it at a folder, and read.

And it does one thing the others don't — it looks at what you actually own and
tells you **what's missing from a series**:

> **Saga** · Image Comics
> You have **6** of **12** — Missing: 4, 6–7, 10–12

No account, no server, no cloud. It reads the `ComicInfo.xml` that most comics
already carry inside the archive, and falls back to reading the filenames when
they don't.

## Features

- **CBZ and CBR**, with natural page ordering (`page2` before `page10`).
- **Browse by folders.** Pick a root folder and navigate it — your library is
  not flattened into one endless grid.
- **Collection tracking.** See which issues of a series you're missing, from
  metadata embedded in your files. Optionally connect to
  [Metron](https://metron.cloud), an open community comic database, to fill in
  the total issue count.
- **Reader** with zoom (100–400%), keyboard navigation, and vertical scrolling
  that turns the page when you reach the bottom.
- **Read tracking.** Comics are marked as read when you finish them, or by hand.
  Mark a whole folder at once.
- **Custom folder covers** — drop a `cover.jpg` in a folder, or set one from the
  app.
- **Multiple libraries**, switchable from a dropdown.
- **Fast with big libraries.** The grid appears instantly whether you have ten
  comics or ten thousand; covers load only for what's on screen and are cached.
- **Small.** A few MB, not the few hundred an Electron app would cost you.
- **Updates itself**, with signed releases.

## Install

Grab the installer from the [latest release](https://github.com/DarioRQ/readingcomics/releases/latest):

| Platform | File |
| --- | --- |
| Windows | `readingcomics_x.y.z_x64-setup.exe` or `.msi` |
| Linux | `.AppImage`, `.deb` or `.rpm` |

> **Windows will warn you** that it doesn't recognise the app. That's because
> the installer isn't signed with a paid code-signing certificate — not because
> anything is wrong with it. Click *More info* → *Run anyway*. Every release is
> built in the open by [GitHub Actions](.github/workflows/release.yml) and
> cryptographically signed for the updater; you can check the workflow yourself.

## Building from source

You'll need [Rust](https://www.rust-lang.org/tools/install), [Node.js](https://nodejs.org/)
and pnpm. On Linux you also need the usual Tauri dependencies plus
`libsecret-1-dev`.

```bash
git clone https://github.com/DarioRQ/readingcomics.git
cd readingcomics
pnpm install
pnpm tauri dev      # run it
pnpm tauri build    # build an installer
```

## How it works

The backend is Rust: it reads the archives, generates cover thumbnails, and
caches them on disk keyed by path + size + modification time, so revisiting a
folder never re-opens a single file. The frontend is React, and it only asks for
the covers of cards that are actually on screen.

Library browsing is confined to the root folder you pick — paths are
canonicalised and checked, and symlinks are not followed.

Anything you connect to is opt-in. Without a Metron account configured, the app
makes **no network requests at all** except checking for its own updates.

## Contributing

Issues and pull requests are welcome. If you hit a comic that won't open, please
open an issue with the error the app shows — that message is there specifically
to make those reports possible.

## License

MIT — see [LICENSE](./LICENSE).

`.cbr` support uses the [`unrar`](https://crates.io/crates/unrar) crate, which
builds the UnRAR source (freeware licence, free to use for extraction, not
OSI-approved). Everything else is MIT.

The bundled typefaces — [Archivo](https://github.com/Omnibus-Type/Archivo) and
[Fraunces](https://github.com/undercasetype/Fraunces) — are under the SIL Open
Font Licence 1.1; their licence texts ship next to the font files in
`src/assets/fonts/`. They are packaged with the app rather than fetched from a
CDN, so the reader keeps working offline and asks nobody for anything on start.
