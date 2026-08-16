# readingcomics

Lector de cómics de escritorio (Windows, Linux, y Android a futuro), open source.
Hecho con [Tauri 2](https://tauri.app) (backend en Rust) + React/TypeScript.

## Estado actual

- Soporta `.cbz` y `.cbr` (más formatos en el roadmap: PDF, EPUB/manga).
- Biblioteca en grid con portadas generadas automáticamente.
- Lector con navegación por teclado (flechas), click, y precarga de la
  siguiente página.
- Auto-actualización integrada (comprueba releases de GitHub y se actualiza
  sola, firmando cada release para verificar que viene de este repo).

## Desarrollo en Windows

Prerequisitos (una sola vez):

1. [Rust](https://www.rust-lang.org/tools/install) (`rustup-init.exe`, toolchain estable).
2. [Node.js](https://nodejs.org/) (LTS) y `pnpm` (`npm install -g pnpm`).
3. [Microsoft C++ Build Tools](https://tauri.app/start/prerequisites/#windows) — necesarios para compilar Rust en Windows. El instalador de Rust te avisa si faltan.
4. WebView2 — ya viene instalado de serie en Windows 10/11.

Clonar y arrancar:

```powershell
git clone https://github.com/DarioRQ/readingcomics.git
cd readingcomics
pnpm install
pnpm tauri dev
```

Esto abre la app en modo desarrollo con hot-reload. Para generar el `.exe`
instalable localmente:

```powershell
pnpm tauri build
```

## Releases y auto-actualización

Las releases se generan solas vía GitHub Actions (`.github/workflows/release.yml`)
al subir un tag:

```bash
git tag v0.1.0
git push --tags
```

Eso compila para Windows y Linux, firma los artefactos con la clave de
firma del proyecto, y publica un *draft release* en GitHub con el
`latest.json` que la app usa para detectar actualizaciones. Repasa el
draft y publícalo cuando quieras que llegue a los usuarios.

Requiere tener configurados estos *secrets* en
`Settings > Secrets and variables > Actions` del repo:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

(La clave privada nunca vive en este repositorio — se generó aparte y solo
existe como secret de GitHub y en tu propio guardado seguro.)

## Estructura

```
src/            frontend React (biblioteca, lector, titlebar, updater UI)
src-tauri/src/  backend Rust (lectura de CBZ/CBR, comandos Tauri)
```

## Licencia

MIT — ver [LICENSE](./LICENSE).

Nota: la lectura de `.cbr` usa el crate [`unrar`](https://crates.io/crates/unrar),
que compila internamente el código fuente de UnRAR (licencia UnRAR, freeware,
no OSI-open-source pero de uso libre para extracción). El resto del proyecto
es MIT.

Las tipografías que se distribuyen con la aplicación —
[Archivo](https://github.com/Omnibus-Type/Archivo) y
[Fraunces](https://github.com/undercasetype/Fraunces)— son SIL Open Font
License 1.1, con el texto de la licencia junto a los ficheros en
`src/assets/fonts/`. Van dentro del paquete y no en un CDN, para que el lector
funcione sin conexión y no le pida nada a nadie al arrancar.
