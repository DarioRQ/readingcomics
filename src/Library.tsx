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
  BookIcon,
  ChevronLeftIcon,
  FolderStackIcon,
  WarningIcon,
  CheckIcon,
  CheckCircleIcon,
  ImageIcon,
} from "./Icons";
import { useLazyInfo, clearInfoCache } from "./useLazyInfo";
import LibraryPicker from "./LibraryPicker";
import SeriesBanner from "./SeriesBanner";
import CardMenu from "./CardMenu";
import ConfirmDialog from "./ConfirmDialog";
import MetronPanel from "./MetronPanel";

/** Nombre de carpeta a partir de su ruta, sirviendo tanto `/` como `\`. */
function baseName(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function FolderCard({
  folder,
  root,
  onOpen,
  onMarkAll,
  onChangeCover,
}: {
  folder: FolderMeta;
  root: string | null;
  onOpen: () => void;
  onMarkAll: () => void;
  onChangeCover: () => void;
}) {
  const { ref, data } = useLazyInfo<FolderInfo>(
    "get_folder_info",
    root,
    folder.path,
  );

  return (
    <div
      className="comic-card comic-card-folder"
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onOpen();
        }
      }}
    >
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

        <CardMenu
          actions={[
            {
              label: "Cambiar portada…",
              icon: <ImageIcon size={15} />,
              onSelect: onChangeCover,
            },
            {
              label: "Marcar como leída",
              icon: <CheckIcon size={15} />,
              onSelect: onMarkAll,
            },
          ]}
        />
      </div>
      <div className="comic-name" title={folder.name}>
        {folder.name}
      </div>
      <div className="comic-pages">
        {data
          ? `${data.comic_count} ${data.comic_count === 1 ? "cómic" : "cómics"}`
          : "…"}
      </div>
    </div>
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
            ? data.meta?.number
              ? `N.º ${data.meta.number} · ${data.page_count} págs.`
              : `${data.page_count} páginas`
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
  const [libraries, setLibraries] = useState<string[]>([]);
  const [listing, setListing] = useState<DirListing | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressMap>({});
  const [metronOpen, setMetronOpen] = useState(false);
  const [confirm, setConfirm] = useState<{
    path: string;
    name: string;
    paths: string[];
  } | null>(null);
  // Se incrementa para forzar el remontado de las tarjetas cuando cambia algo
  // que ya estaba cacheado, como la portada de una carpeta.
  const [cardVersion, setCardVersion] = useState(0);

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

  /** Cuenta lo que hay dentro y pide confirmación antes de marcar nada. */
  const askMarkFolder = useCallback(
    async (folderPath: string, name: string) => {
      if (!root) return;
      try {
        const paths = await invoke<string[]>("list_comics_deep", {
          root,
          path: folderPath,
        });
        if (paths.length > 0) setConfirm({ path: folderPath, name, paths });
      } catch {
        // Sin lista no hay nada que confirmar.
      }
    },
    [root],
  );

  const applyMarkFolder = useCallback(async () => {
    if (!confirm) return;
    const { paths } = confirm;
    setConfirm(null);
    try {
      await invoke("set_many_read", { paths, read: true });
      setProgress((p) => {
        const next = { ...p };
        for (const path of paths) {
          next[path] = { read: true, last_page: p[path]?.last_page ?? 0 };
        }
        return next;
      });
    } catch {
      // Si falla, el progreso se queda como estaba.
    }
  }, [confirm]);

  /** Elige una imagen y la deja como portada de la carpeta. */
  const changeFolderCover = useCallback(
    async (folderPath: string) => {
      if (!root) return;
      const picked = await open({
        multiple: false,
        filters: [
          { name: "Imágenes", extensions: ["jpg", "jpeg", "png", "webp", "bmp"] },
        ],
      });
      if (!picked || Array.isArray(picked)) return;
      try {
        await invoke("set_folder_cover", {
          root,
          path: folderPath,
          image: picked,
        });
        clearInfoCache();
        setCardVersion((v) => v + 1);
      } catch {
        // La portada anterior se queda como estaba.
      }
    },
    [root],
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

  // Al arrancar, recuperamos la última biblioteca usada y la lista guardada.
  useEffect(() => {
    let cancelled = false;
    invoke<AppConfig>("load_config")
      .then((cfg) => {
        if (cancelled) return;
        setLibraries(cfg.libraries ?? []);
        if (cfg.library_root) browse(cfg.library_root);
        else setLoading(false);
      })
      .catch(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [browse]);

  const persist = useCallback((library_root: string | null, libs: string[]) => {
    invoke("save_config", {
      config: { library_root, libraries: libs },
    }).catch(() => {});
  }, []);

  /** Cambia a una biblioteca ya guardada. */
  const selectLibrary = useCallback(
    (dir: string) => {
      clearInfoCache();
      browse(dir);
      persist(dir, libraries);
    },
    [browse, libraries, persist],
  );

  /** Añade una biblioteca nueva desde el selector de carpetas. */
  const addLibrary = useCallback(async () => {
    const dir = await open({ directory: true, multiple: false });
    if (!dir || Array.isArray(dir)) return;
    // Sin duplicados: volver a elegir una ya guardada solo la abre.
    const next = libraries.includes(dir) ? libraries : [...libraries, dir];
    setLibraries(next);
    clearInfoCache();
    await browse(dir);
    persist(dir, next);
  }, [browse, libraries, persist]);

  /** Quita una biblioteca de la lista. No toca nada del disco. */
  const removeLibrary = useCallback(
    (dir: string) => {
      const next = libraries.filter((l) => l !== dir);
      setLibraries(next);
      // Si se quita la que está abierta, la vista se queda sin biblioteca.
      const stillOpen = dir === root ? null : root;
      if (dir === root) {
        setRoot(null);
        setListing(null);
      }
      persist(stillOpen, next);
    },
    [libraries, root, persist],
  );

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

        <LibraryPicker
          current={root}
          libraries={libraries}
          onSelect={selectLibrary}
          onAdd={addLibrary}
          onRemove={removeLibrary}
        />

        {listing && !loading && !isRoot && (
          <span className="library-path" title={listing.path}>
            {baseName(listing.path)}
          </span>
        )}
        {loading && <span className="loading-text">Cargando…</span>}
        {error && <span className="error-text">{error}</span>}

        <button
          className="ghost-btn toolbar-end"
          onClick={() => setMetronOpen(true)}
          title="Conectar con la base de datos Metron"
        >
          Metron
        </button>
      </div>

      {metronOpen && <MetronPanel onClose={() => setMetronOpen(false)} />}

      {confirm && (
        <ConfirmDialog
          title="Marcar como leída"
          message={`Se marcarán como leídos los ${confirm.paths.length} cómics que hay dentro de «${confirm.name}», incluidas sus subcarpetas.`}
          confirmLabel="Marcar todos"
          onConfirm={applyMarkFolder}
          onCancel={() => setConfirm(null)}
        />
      )}

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

      {!loading && listing && root && listing.comics.length > 0 && (
        <SeriesBanner root={root} path={listing.path} />
      )}

      {!loading && listing && (
        <div className="library-grid">
          {listing.folders.map((folder) => (
            <FolderCard
              key={`${folder.path}:${cardVersion}`}
              folder={folder}
              root={root}
              onOpen={() => root && browse(root, folder.path)}
              onMarkAll={() => askMarkFolder(folder.path, folder.name)}
              onChangeCover={() => changeFolderCover(folder.path)}
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
