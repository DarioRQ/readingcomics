import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, ComicMeta, DirListing } from "./types";
import {
  FolderIcon,
  BookIcon,
  ChevronLeftIcon,
  FolderStackIcon,
} from "./Icons";

/** Nombre de carpeta a partir de su ruta, sirviendo tanto `/` como `\`. */
function baseName(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export default function Library({
  onOpenComic,
}: {
  onOpenComic: (comic: ComicMeta) => void;
}) {
  const [root, setRoot] = useState<string | null>(null);
  const [listing, setListing] = useState<DirListing | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const browse = useCallback(async (libRoot: string, path?: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<DirListing>("list_dir", {
        root: libRoot,
        path: path ?? null,
      });
      setListing(result);
      setRoot(libRoot);
    } catch (e) {
      setListing(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Al arrancar, recuperamos la última biblioteca usada.
  useEffect(() => {
    let cancelled = false;
    invoke<AppConfig>("load_config")
      .then((cfg) => {
        if (cancelled) return;
        if (cfg.library_root) browse(cfg.library_root);
        else setLoading(false);
      })
      .catch(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [browse]);

  const pickFolder = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (!dir || Array.isArray(dir)) return;
    await browse(dir);
    invoke("save_config", { config: { library_root: dir } }).catch(() => {});
  };

  const isRoot = listing !== null && listing.parent === null;
  const empty =
    listing !== null &&
    listing.folders.length === 0 &&
    listing.comics.length === 0;

  return (
    <div className="library">
      <div className="library-toolbar">
        {root && listing && !isRoot ? (
          <button
            className="ghost-btn with-icon"
            onClick={() => browse(root, listing.parent ?? undefined)}
          >
            <ChevronLeftIcon size={16} />
            Atrás
          </button>
        ) : null}

        <button className="primary-btn with-icon" onClick={pickFolder}>
          <FolderIcon size={16} />
          {root ? "Cambiar biblioteca" : "Elegir carpeta de cómics"}
        </button>

        {listing && !loading && (
          <span className="library-path" title={listing.path}>
            {isRoot ? listing.path : baseName(listing.path)}
          </span>
        )}
        {loading && <span className="loading-text">Cargando…</span>}
        {error && <span className="error-text">{error}</span>}
      </div>

      {!loading && !listing && !error && (
        <div className="empty-state">
          <p>Ninguna biblioteca cargada.</p>
          <p className="empty-hint">
            Elige una carpeta con archivos .cbz / .cbr para empezar.
          </p>
        </div>
      )}

      {!loading && empty && (
        <div className="empty-state">
          <p>Esta carpeta no contiene cómics.</p>
        </div>
      )}

      {!loading && listing && (
        <div className="library-grid">
          {listing.folders.map((folder) => (
            <button
              key={folder.path}
              className="comic-card"
              onClick={() => root && browse(root, folder.path)}
            >
              <div className="comic-cover">
                {folder.cover ? (
                  <img src={folder.cover} alt={folder.name} loading="lazy" />
                ) : (
                  <div className="comic-cover-placeholder">
                    <FolderStackIcon size={32} />
                  </div>
                )}
                <span className="folder-badge">
                  <FolderStackIcon size={13} />
                </span>
              </div>
              <div className="comic-name" title={folder.name}>
                {folder.name}
              </div>
              <div className="comic-pages">
                {folder.comic_count} {folder.comic_count === 1 ? "cómic" : "cómics"}
              </div>
            </button>
          ))}

          {listing.comics.map((comic) => (
            <button
              key={comic.path}
              className="comic-card"
              onClick={() => onOpenComic(comic)}
            >
              <div className="comic-cover">
                {comic.cover ? (
                  <img src={comic.cover} alt={comic.name} loading="lazy" />
                ) : (
                  <div className="comic-cover-placeholder">
                    <BookIcon size={32} />
                  </div>
                )}
              </div>
              <div className="comic-name" title={comic.name}>
                {comic.name}
              </div>
              <div className="comic-pages">{comic.page_count} páginas</div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
