import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, ComicMeta } from "./types";
import { FolderIcon, BookIcon } from "./Icons";

export default function Library({
  onOpenComic,
}: {
  onOpenComic: (comic: ComicMeta) => void;
}) {
  const [library, setLibrary] = useState<ComicMeta[]>([]);
  const [root, setRoot] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const scan = useCallback(async (dir: string) => {
    setLoading(true);
    setError(null);
    try {
      const comics = await invoke<ComicMeta[]>("scan_library", { root: dir });
      setLibrary(comics);
      setRoot(dir);
    } catch (e) {
      setLibrary([]);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Al arrancar, recuperamos la última carpeta usada y la escaneamos sola.
  useEffect(() => {
    let cancelled = false;
    invoke<AppConfig>("load_config")
      .then((cfg) => {
        if (cancelled) return;
        if (cfg.library_root) scan(cfg.library_root);
        else setLoading(false);
      })
      .catch(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [scan]);

  const pickFolder = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (!dir || Array.isArray(dir)) return;
    await scan(dir);
    // Se guarda aunque el escaneo falle: si el usuario la eligió a propósito,
    // querrá reintentarla la próxima vez, no volver a buscarla.
    invoke("save_config", { config: { library_root: dir } }).catch(() => {});
  };

  return (
    <div className="library">
      <div className="library-toolbar">
        <button className="primary-btn with-icon" onClick={pickFolder}>
          <FolderIcon size={16} />
          {root ? "Cambiar carpeta" : "Elegir carpeta de cómics"}
        </button>
        {root && !loading && (
          <span className="library-path" title={root}>
            {root}
          </span>
        )}
        {loading && <span className="loading-text">Escaneando…</span>}
        {error && <span className="error-text">{error}</span>}
      </div>

      {!loading && library.length === 0 && !error && (
        <div className="empty-state">
          <p>
            {root
              ? "No se encontró ningún cómic en esta carpeta."
              : "Ninguna biblioteca cargada."}
          </p>
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
    </div>
  );
}
