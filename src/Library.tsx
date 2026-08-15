import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  ComicInfo,
  ComicMeta,
  DirListing,
  FolderInfo,
  FolderMeta,
  ProgressMap,
} from "./types";
import {
  FolderIcon,
  BookIcon,
  ChevronLeftIcon,
  FolderStackIcon,
  WarningIcon,
  CheckIcon,
  CheckCircleIcon,
} from "./Icons";
import { useLazyInfo, clearInfoCache } from "./useLazyInfo";

/** Nombre de carpeta a partir de su ruta, sirviendo tanto `/` como `\`. */
function baseName(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function FolderCard({
  folder,
  root,
  onOpen,
}: {
  folder: FolderMeta;
  root: string | null;
  onOpen: () => void;
}) {
  const { ref, data } = useLazyInfo<FolderInfo>(
    "get_folder_info",
    root,
    folder.path,
  );

  return (
    <button className="comic-card comic-card-folder" onClick={onOpen}>
      <div className="comic-cover" ref={ref}>
        {data?.cover ? (
          <img src={data.cover} alt={folder.name} loading="lazy" />
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
        {data
          ? `${data.comic_count} ${data.comic_count === 1 ? "cómic" : "cómics"}`
          : "…"}
      </div>
    </button>
  );
}

function ComicCard({
  comic,
  root,
  read,
  onOpen,
  onToggleRead,
}: {
  comic: ComicMeta;
  root: string | null;
  read: boolean;
  onOpen: () => void;
  onToggleRead: () => void;
}) {
  const { ref, data } = useLazyInfo<ComicInfo>(
    "get_comic_info",
    root,
    comic.path,
  );
  const broken = Boolean(data?.error);

  return (
    // Es un div y no un button porque dentro lleva el botón de "leído", y
    // anidar botones no es HTML válido.
    <div
      className={`comic-card${broken ? " comic-card-broken" : ""}${
        read ? " comic-card-read" : ""
      }`}
      role="button"
      tabIndex={broken ? -1 : 0}
      onClick={broken ? undefined : onOpen}
      onKeyDown={(e) => {
        if (!broken && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          onOpen();
        }
      }}
      title={data?.error ?? comic.name}
    >
      <div className="comic-cover" ref={ref}>
        {data?.cover ? (
          <img src={data.cover} alt={comic.name} loading="lazy" />
        ) : (
          <div className="comic-cover-placeholder">
            {broken ? <WarningIcon size={30} /> : <BookIcon size={32} />}
          </div>
        )}

        {read && (
          <span className="read-badge" aria-hidden="true">
            <CheckCircleIcon size={14} />
          </span>
        )}

        {!broken && (
          <button
            className={`read-toggle${read ? " read-toggle-on" : ""}`}
            onClick={(e) => {
              e.stopPropagation(); // que no abra el cómic al marcarlo
              onToggleRead();
            }}
            aria-label={read ? "Marcar como no leído" : "Marcar como leído"}
            title={read ? "Marcar como no leído" : "Marcar como leído"}
          >
            <CheckIcon size={14} />
          </button>
        )}
      </div>
      <div className="comic-name" title={comic.name}>
        {comic.name}
      </div>
      <div className={`comic-pages${broken ? " comic-pages-error" : ""}`}>
        {broken
          ? "No se pudo leer"
          : data
            ? `${data.page_count} páginas`
            : "…"}
      </div>
    </div>
  );
}

export default function Library({
  onOpenComic,
  progressVersion,
}: {
  onOpenComic: (comic: ComicMeta) => void;
  progressVersion: number;
}) {
  const [root, setRoot] = useState<string | null>(null);
  const [listing, setListing] = useState<DirListing | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressMap>({});

  // Se recarga al volver de leer un cómic, para reflejar lo recién terminado.
  useEffect(() => {
    invoke<ProgressMap>("get_progress")
      .then(setProgress)
      .catch(() => {});
  }, [progressVersion]);

  const toggleRead = useCallback(
    (path: string, current: boolean) => {
      const next = !current;
      // Optimista: la tarjeta responde al instante y el disco va detrás.
      setProgress((p) => ({
        ...p,
        [path]: { read: next, last_page: next ? (p[path]?.last_page ?? 0) : 0 },
      }));
      invoke("set_read", { path, read: next }).catch(() => {
        setProgress((p) => ({ ...p, [path]: { ...p[path], read: current } }));
      });
    },
    [],
  );

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
    clearInfoCache();
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
            <FolderCard
              key={folder.path}
              folder={folder}
              root={root}
              onOpen={() => root && browse(root, folder.path)}
            />
          ))}

          {listing.comics.map((comic) => {
            const isRead = progress[comic.path]?.read ?? false;
            return (
              <ComicCard
                key={comic.path}
                comic={comic}
                root={root}
                read={isRead}
                onOpen={() => onOpenComic(comic)}
                onToggleRead={() => toggleRead(comic.path, isRead)}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
