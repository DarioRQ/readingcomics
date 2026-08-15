import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ComicMeta } from "./types";
import {
  ChevronLeftIcon,
  CheckIcon,
  ZoomInIcon,
  ZoomOutIcon,
} from "./Icons";

/** 1 = página completa en pantalla. */
const ZOOM_MIN = 1;
const ZOOM_MAX = 4;
const ZOOM_STEP = 0.25;
/** Cuánto avanza la vista con cada pulsación de flecha estando ampliado. */
const SCROLL_STEP = 120;
/** Margen para dar por hecho que ya se llegó al final de la página. */
const EDGE_SLACK = 2;

const clampZoom = (z: number) =>
  Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(z * 100) / 100));

export default function Reader({
  comic,
  onClose,
}: {
  comic: ComicMeta;
  onClose: () => void;
}) {
  const [pages, setPages] = useState<string[]>([]);
  const [index, setIndex] = useState(0);
  const [image, setImage] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [read, setRead] = useState(false);
  const [zoom, setZoom] = useState(1);
  const cache = useRef<Map<string, string>>(new Map());
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    setPages([]);
    setIndex(0);
    invoke<string[]>("open_comic", { path: comic.path }).then((names) => {
      if (!cancelled) setPages(names);
    });
    return () => {
      cancelled = true;
    };
  }, [comic.path]);

  const loadPage = useCallback(
    async (i: number) => {
      const name = pages[i];
      if (!name) return null;
      const key = `${comic.path}::${name}`;
      const cached = cache.current.get(key);
      if (cached) return cached;
      const data = await invoke<string>("get_page", {
        path: comic.path,
        name,
      });
      cache.current.set(key, data);
      return data;
    },
    [pages, comic.path],
  );

  useEffect(() => {
    if (!pages.length) return;
    let cancelled = false;
    setLoading(true);
    loadPage(index).then((data) => {
      if (cancelled) return;
      setImage(data);
      setLoading(false);
    });
    if (index + 1 < pages.length) loadPage(index + 1);
    return () => {
      cancelled = true;
    };
  }, [index, pages, loadPage]);

  // Guarda por dónde vas. El backend marca el cómic como leído solo al
  // alcanzar la última página, y nos devuelve si ya cuenta como leído.
  useEffect(() => {
    if (!pages.length) return;
    invoke<boolean>("set_progress", {
      path: comic.path,
      page: index,
      total: pages.length,
    })
      .then(setRead)
      .catch(() => {});
  }, [index, pages.length, comic.path]);

  const toggleRead = () => {
    const next = !read;
    setRead(next);
    invoke("set_read", { path: comic.path, read: next }).catch(() =>
      setRead(!next),
    );
  };

  /**
   * Cambia de página dejando la vista en el borde por el que se entra: al
   * avanzar se empieza arriba, y al retroceder se aparece abajo, que es por
   * donde se venía leyendo.
   */
  const goTo = useCallback(
    (next: number, landAt: "top" | "bottom") => {
      setIndex((i) => {
        const target = Math.min(Math.max(next, 0), pages.length - 1);
        if (target === i) return i;
        requestAnimationFrame(() => {
          const el = scrollRef.current;
          if (el) el.scrollTop = landAt === "top" ? 0 : el.scrollHeight;
        });
        return target;
      });
    },
    [pages.length],
  );

  const goNext = useCallback(() => goTo(index + 1, "top"), [goTo, index]);
  const goPrev = useCallback(() => goTo(index - 1, "top"), [goTo, index]);

  /**
   * Flechas verticales: mientras quede página por debajo, desplazan dentro de
   * ella; al llegar al borde, pasan de página. Si la página cabe entera, pasan
   * directamente.
   */
  const scrollOrTurn = useCallback(
    (dir: 1 | -1) => {
      const el = scrollRef.current;
      if (el) {
        const max = el.scrollHeight - el.clientHeight;
        const atEnd =
          dir === 1 ? el.scrollTop >= max - EDGE_SLACK : el.scrollTop <= EDGE_SLACK;
        if (max > EDGE_SLACK && !atEnd) {
          el.scrollBy({ top: dir * SCROLL_STEP });
          return;
        }
      }
      if (dir === 1) goTo(index + 1, "top");
      else goTo(index - 1, "bottom");
    },
    [goTo, index],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          scrollOrTurn(1);
          break;
        case "ArrowUp":
          e.preventDefault();
          scrollOrTurn(-1);
          break;
        case " ":
          e.preventDefault();
          scrollOrTurn(1);
          break;
        case "ArrowRight":
          goNext();
          break;
        case "ArrowLeft":
          goPrev();
          break;
        case "+":
        case "=":
          setZoom((z) => clampZoom(z + ZOOM_STEP));
          break;
        case "-":
          setZoom((z) => clampZoom(z - ZOOM_STEP));
          break;
        case "0":
          setZoom(1);
          break;
        case "Escape":
          onClose();
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [scrollOrTurn, goNext, goPrev, onClose]);

  // Ctrl + rueda para ampliar, como en cualquier visor.
  const onWheel = (e: React.WheelEvent) => {
    if (!e.ctrlKey) return;
    e.preventDefault();
    setZoom((z) => clampZoom(z + (e.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP)));
  };

  const zoomed = zoom > 1;

  return (
    <div className="reader">
      <div className="reader-topbar">
        <button className="ghost-btn with-icon" onClick={onClose}>
          <ChevronLeftIcon size={16} />
          Biblioteca
        </button>
        <span className="reader-title">{comic.name}</span>

        <div className="reader-zoom">
          <button
            className="icon-btn"
            onClick={() => setZoom((z) => clampZoom(z - ZOOM_STEP))}
            disabled={zoom <= ZOOM_MIN}
            aria-label="Alejar"
            title="Alejar (−)"
          >
            <ZoomOutIcon size={15} />
          </button>
          <button
            className="zoom-level"
            onClick={() => setZoom(1)}
            title="Restablecer zoom (0)"
          >
            {Math.round(zoom * 100)}%
          </button>
          <button
            className="icon-btn"
            onClick={() => setZoom((z) => clampZoom(z + ZOOM_STEP))}
            disabled={zoom >= ZOOM_MAX}
            aria-label="Ampliar"
            title="Ampliar (+)"
          >
            <ZoomInIcon size={15} />
          </button>
        </div>

        <span className="reader-counter">
          {pages.length ? `${index + 1} / ${pages.length}` : ""}
        </span>
        <button
          className={`ghost-btn with-icon${read ? " ghost-btn-on" : ""}`}
          onClick={toggleRead}
          title={read ? "Marcar como no leído" : "Marcar como leído"}
        >
          <CheckIcon size={15} />
          {read ? "Leído" : "Marcar leído"}
        </button>
      </div>

      <div
        className="reader-stage"
        ref={scrollRef}
        onWheel={onWheel}
        tabIndex={-1}
      >
        {/* Con la página ampliada las zonas de clic estorban: al arrastrar
            para moverse acabarías cambiando de página sin querer. */}
        {!zoomed && (
          <>
            <div className="reader-zone reader-zone-left" onClick={goPrev} />
            <div className="reader-zone reader-zone-right" onClick={goNext} />
          </>
        )}

        {loading || !image ? (
          <div className="reader-loading">Cargando…</div>
        ) : (
          <img
            className="reader-page"
            src={image}
            alt={`Página ${index + 1}`}
            draggable={false}
            style={
              zoomed
                ? {
                    height: `${zoom * 100}%`,
                    width: "auto",
                    maxWidth: "none",
                    maxHeight: "none",
                  }
                : undefined
            }
          />
        )}
      </div>
    </div>
  );
}
