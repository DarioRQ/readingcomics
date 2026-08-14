import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { ComicMeta } from "./types";

export default function Library({
  onOpenComic,
}: {
  onOpenComic: (comic: ComicMeta) => void;
}) {
  const [library, setLibrary] = useState<ComicMeta[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pickFolder = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (!dir || Array.isArray(dir)) return;
    setLoading(true);
    setError(null);
    try {
      const comics = await invoke<ComicMeta[]>("scan_library", { root: dir });
      setLibrary(comics);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="library">
      <div className="library-toolbar">
        <button className="primary-btn" onClick={pickFolder}>
          {library.length ? "Cambiar carpeta" : "Elegir carpeta de cómics"}
        </button>
        {loading && <span className="loading-text">Escaneando…</span>}
        {error && <span className="error-text">{error}</span>}
      </div>

      {!loading && library.length === 0 && !error && (
        <div className="empty-state">
          <p>Ninguna biblioteca cargada.</p>
          <p className="empty-hint">
            Elige una carpeta con archivos .cbz / .cbr para empezar.
          </p>
        </div>
      )}

      <div className="library-grid">
        {library.map((comic) => (
          <button
            key={comic.path}
            className="comic-card"
            onClick={() => onOpenComic(comic)}
          >
            <div className="comic-cover">
              {comic.cover ? (
                <img src={comic.cover} alt={comic.name} loading="lazy" />
              ) : (
                <div className="comic-cover-placeholder">?</div>
              )}
            </div>
            <div className="comic-name" title={comic.name}>
              {comic.name}
            </div>
            <div className="comic-pages">{comic.page_count} páginas</div>
          </button>
        ))}
      </div>
    </div>
  );
}
