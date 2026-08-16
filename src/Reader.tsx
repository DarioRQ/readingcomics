import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ComicMeta, FitMode, ProgressMap, ReaderPrefs } from "./types";
import ThumbStrip from "./ThumbStrip";
import {
  ChevronLeftIcon,
  CheckIcon,
  ZoomInIcon,
  ZoomOutIcon,
  FitPageIcon,
  FitWidthIcon,
  FitHeightIcon,
  OriginalSizeIcon,
  SpreadIcon,
  MangaIcon,
  RotateIcon,
  ThumbsIcon,
  FullscreenIcon,
  ExitFullscreenIcon,
} from "./Icons";

/** Multiplicador sobre el tamaño que impone el modo de ajuste. */
const ZOOM_MIN = 0.25;
const ZOOM_MAX = 4;
const ZOOM_STEP = 0.25;
/** Cuánto avanza la vista con cada pulsación de flecha estando ampliado. */
const SCROLL_STEP = 120;
/** Margen para dar por hecho que ya se llegó al final de la página. */
const EDGE_SLACK = 2;
/** Separación entre las dos páginas de un pliego, en píxeles de imagen. */
const SPREAD_GAP = 4;

const clampZoom = (z: number) =>
  Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(z * 100) / 100));

const DEFAULT_PREFS: ReaderPrefs = {
  fit: "page",
  spread: false,
  cover_alone: true,
  manga: false,
  thumbs: false,
};

const FIT_MODES: { mode: FitMode; label: string; key: string; icon: typeof FitPageIcon }[] = [
  { mode: "page", label: "Página entera", key: "B", icon: FitPageIcon },
  { mode: "width", label: "Ajustar al ancho", key: "W", icon: FitWidthIcon },
  { mode: "height", label: "Ajustar al alto", key: "H", icon: FitHeightIcon },
  { mode: "original", label: "Tamaño original", key: "O", icon: OriginalSizeIcon },
];

/** Una página ya descargada, con su tamaño real: hace falta para calcular el
 *  encaje antes de pintarla, y para que el pliego cuadre. */
type Loaded = { src: string; w: number; h: number };

export default function Reader({
  comic,
  onClose,
}: {
  comic: ComicMeta;
  onClose: () => void;
}) {
  const [pages, setPages] = useState<string[]>([]);
  const [index, setIndex] = useState(0);
  const [imgs, setImgs] = useState<Loaded[]>([]);
  const [loading, setLoading] = useState(true);
  const [read, setRead] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [rotation, setRotation] = useState(0);
  const [prefs, setPrefs] = useState<ReaderPrefs>(DEFAULT_PREFS);
  const [full, setFull] = useState(false);
  const [stage, setStage] = useState({ w: 0, h: 0 });
  const [goingTo, setGoingTo] = useState<string | null>(null);
  const cache = useRef<Map<string, Loaded>>(new Map());
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    setPages([]);
    setIndex(0);
    setRotation(0);

    // Se abre por donde se dejó. El progreso ya se guardaba, pero nadie lo
    // leía al abrir: el cómic empezaba siempre por la primera página. Un cómic
    // terminado sí vuelve al principio, que es lo que se quiere al releerlo.
    Promise.all([
      invoke<string[]>("open_comic", { path: comic.path }),
      invoke<ProgressMap>("get_progress").catch(() => ({}) as ProgressMap),
    ]).then(([names, progress]) => {
      if (cancelled) return;
      setPages(names);
      const state = progress[comic.path];
      if (state && !state.read && state.last_page > 0) {
        setIndex(Math.min(state.last_page, names.length - 1));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [comic.path]);

  /* ---------- Preferencias ---------- */

  useEffect(() => {
    invoke<ReaderPrefs>("load_reader_prefs").then(setPrefs).catch(() => {});
  }, []);

  const update = useCallback((patch: Partial<ReaderPrefs>) => {
    setPrefs((p) => {
      const next = { ...p, ...patch };
      invoke("save_reader_prefs", { prefs: next }).catch(() => {});
      return next;
    });
  }, []);

  /* ---------- Páginas ---------- */

  const loadPage = useCallback(
    async (i: number): Promise<Loaded | null> => {
      const name = pages[i];
      if (!name) return null;
      const key = `${comic.path}::${name}`;
      const hit = cache.current.get(key);
      if (hit) return hit;

      const src = await invoke<string>("get_page", { path: comic.path, name });
      // El tamaño real de la imagen se mide antes de pintarla: sin él no se
      // puede saber cuánto hay que escalar para que encaje, y menos aún cuadrar
      // dos páginas distintas en un mismo pliego.
      const size = await new Promise<{ w: number; h: number }>((resolve) => {
        const probe = new Image();
        probe.onload = () =>
          resolve({ w: probe.naturalWidth || 1, h: probe.naturalHeight || 1 });
        probe.onerror = () => resolve({ w: 1, h: 1 });
        probe.src = src;
      });

      const loaded = { src, ...size };
      cache.current.set(key, loaded);
      return loaded;
    },
    [pages, comic.path],
  );

  /**
   * Reparto de páginas en pliegos. En doble página la portada va sola si el
   * usuario lo pide: si no, el resto del cómic queda emparejado al revés —la
   * izquierda de un pliego con la derecha del siguiente— y se lee fatal.
   */
  const spreads = useMemo(() => {
    const out: number[][] = [];
    if (!pages.length) return out;
    if (!prefs.spread) {
      for (let i = 0; i < pages.length; i++) out.push([i]);
      return out;
    }
    let i = 0;
    if (prefs.cover_alone) {
      out.push([0]);
      i = 1;
    }
    for (; i < pages.length; i += 2) {
      out.push(i + 1 < pages.length ? [i, i + 1] : [i]);
    }
    return out;
  }, [pages.length, prefs.spread, prefs.cover_alone]);

  const current = useMemo(() => {
    const at = spreads.findIndex((s) => s.includes(index));
    return at < 0 ? 0 : at;
  }, [spreads, index]);

  const shown = spreads[current] ?? [];

  useEffect(() => {
    if (!spreads.length) return;
    let cancelled = false;
    setLoading(true);
    const wanted = spreads[current] ?? [];
    Promise.all(wanted.map(loadPage)).then((res) => {
      if (cancelled) return;
      setImgs(res.filter((r): r is Loaded => r !== null));
      setLoading(false);
    });
    // El pliego siguiente se va cargando por detrás.
    (spreads[current + 1] ?? []).forEach(loadPage);
    return () => {
      cancelled = true;
    };
  }, [current, spreads, loadPage]);

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

  /* ---------- Medidas y encaje ---------- */

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const measure = () => setStage({ w: el.clientWidth, h: el.clientHeight });
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  /**
   * Tamaño final de lo que se pinta.
   *
   * Se calcula aquí en vez de dejárselo al CSS porque hay que combinar tres
   * cosas que se estorban entre sí: el modo de ajuste, la rotación —que
   * intercambia ancho y alto— y el pliego de dos páginas, que se mide como una
   * sola imagen ancha.
   */
  const layout = useMemo(() => {
    if (!imgs.length || !stage.w || !stage.h) return null;

    // Las dos páginas de un pliego se igualan en altura antes de sumar anchos:
    // rara vez tienen exactamente el mismo tamaño.
    const tall = Math.max(...imgs.map((i) => i.h));
    const widths = imgs.map((i) => (i.w * tall) / i.h);
    const gap = imgs.length > 1 ? SPREAD_GAP : 0;
    const contentW = widths.reduce((a, b) => a + b, 0) + gap * (imgs.length - 1);
    const contentH = tall;

    const turned = rotation === 90 || rotation === 270;
    const boxW = turned ? contentH : contentW;
    const boxH = turned ? contentW : contentH;

    let scale = 1;
    if (prefs.fit === "page") scale = Math.min(stage.w / boxW, stage.h / boxH);
    else if (prefs.fit === "width") scale = stage.w / boxW;
    else if (prefs.fit === "height") scale = stage.h / boxH;
    scale *= zoom;

    return {
      scale,
      widths,
      contentW: contentW * scale,
      contentH: contentH * scale,
      boxW: boxW * scale,
      boxH: boxH * scale,
      gap: gap * scale,
    };
  }, [imgs, stage, rotation, prefs.fit, zoom]);

  /* ---------- Navegación ---------- */

  const goToSpread = useCallback(
    (target: number, landAt: "top" | "bottom") => {
      const clamped = Math.min(Math.max(target, 0), spreads.length - 1);
      const first = spreads[clamped]?.[0];
      if (first === undefined) return;
      requestAnimationFrame(() => {
        const el = scrollRef.current;
        if (el) el.scrollTop = landAt === "top" ? 0 : el.scrollHeight;
      });
      setIndex(first);
    },
    [spreads],
  );

  const goNext = useCallback(
    () => goToSpread(current + 1, "top"),
    [goToSpread, current],
  );
  const goPrev = useCallback(
    () => goToSpread(current - 1, "top"),
    [goToSpread, current],
  );

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
      if (dir === 1) goToSpread(current + 1, "top");
      else goToSpread(current - 1, "bottom");
    },
    [goToSpread, current],
  );

  /* ---------- Pantalla completa ---------- */

  const setFullscreen = useCallback(async (on: boolean) => {
    setFull(on);
    try {
      await getCurrentWindow().setFullscreen(on);
    } catch {
      // Si el sistema no deja, al menos la interfaz se queda coherente.
      setFull(false);
    }
  }, []);

  // Salir del lector no puede dejar la ventana en pantalla completa.
  useEffect(() => {
    return () => {
      getCurrentWindow()
        .setFullscreen(false)
        .catch(() => {});
    };
  }, []);

  const rotate = (delta: number) =>
    setRotation((r) => (((r + delta) % 360) + 360) % 360);

  /* ---------- Teclado ---------- */

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Con el cursor en el campo de "ir a la página", las teclas son texto.
      if (e.target instanceof HTMLInputElement) return;

      // En manga las flechas horizontales van al revés, como el propio cómic.
      const forward = prefs.manga ? "ArrowLeft" : "ArrowRight";
      const back = prefs.manga ? "ArrowRight" : "ArrowLeft";

      switch (e.key) {
        case "ArrowDown":
        case " ":
          e.preventDefault();
          scrollOrTurn(1);
          break;
        case "ArrowUp":
          e.preventDefault();
          scrollOrTurn(-1);
          break;
        case forward:
          goNext();
          break;
        case back:
          goPrev();
          break;
        case "Home":
          goToSpread(0, "top");
          break;
        case "End":
          goToSpread(spreads.length - 1, "top");
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
        case "b":
          update({ fit: "page" });
          break;
        case "w":
          update({ fit: "width" });
          break;
        case "h":
          update({ fit: "height" });
          break;
        case "o":
          update({ fit: "original" });
          break;
        case "d":
          update({ spread: !prefs.spread });
          break;
        case "m":
          update({ manga: !prefs.manga });
          break;
        case "t":
          update({ thumbs: !prefs.thumbs });
          break;
        case "r":
          rotate(e.shiftKey ? -90 : 90);
          break;
        case "g":
          e.preventDefault();
          setGoingTo(String(index + 1));
          break;
        case "F11":
        case "f":
          e.preventDefault();
          setFullscreen(!full);
          break;
        case "Escape":
          if (full) setFullscreen(false);
          else onClose();
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    scrollOrTurn,
    goNext,
    goPrev,
    goToSpread,
    spreads.length,
    onClose,
    prefs,
    update,
    full,
    setFullscreen,
    index,
  ]);

  // Ctrl + rueda para ampliar, como en cualquier visor.
  const onWheel = (e: React.WheelEvent) => {
    if (!e.ctrlKey) return;
    e.preventDefault();
    setZoom((z) => clampZoom(z + (e.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP)));
  };

  const submitGoto = (e: React.FormEvent) => {
    e.preventDefault();
    const n = Number.parseInt(goingTo ?? "", 10);
    setGoingTo(null);
    if (Number.isNaN(n)) return;
    const page = Math.min(Math.max(n - 1, 0), pages.length - 1);
    const at = spreads.findIndex((s) => s.includes(page));
    goToSpread(at < 0 ? 0 : at, "top");
  };

  // Con la página ampliada a mano, las zonas de clic estorban: al arrastrar
  // para moverte acabarías cambiando de página sin querer. Ajustar al ancho no
  // cuenta como ampliar: ahí sigue siendo cómodo pasar página con un clic.
  const zoomed = zoom > 1;

  return (
    <div className={`reader${full ? " reader-full" : ""}`}>
      <div className="reader-topbar">
        <button className="ghost-btn with-icon" onClick={onClose}>
          <ChevronLeftIcon size={16} />
          Biblioteca
        </button>
        <span className="reader-title">{comic.name}</span>

        <div className="reader-tools">
          {FIT_MODES.map(({ mode, label, key, icon: Icon }) => (
            <button
              key={mode}
              className={`icon-btn${prefs.fit === mode ? " icon-btn-on" : ""}`}
              onClick={() => update({ fit: mode })}
              aria-label={label}
              aria-pressed={prefs.fit === mode}
              title={`${label} (${key})`}
            >
              <Icon size={15} />
            </button>
          ))}
        </div>

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

        <div className="reader-tools">
          <button
            className={`icon-btn${prefs.spread ? " icon-btn-on" : ""}`}
            onClick={() => update({ spread: !prefs.spread })}
            aria-pressed={prefs.spread}
            aria-label="Doble página"
            title="Doble página (D)"
          >
            <SpreadIcon size={15} />
          </button>
          {prefs.spread && (
            <button
              className={`icon-btn${prefs.cover_alone ? " icon-btn-on" : ""}`}
              onClick={() => update({ cover_alone: !prefs.cover_alone })}
              aria-pressed={prefs.cover_alone}
              aria-label="La portada va sola"
              title="La portada va sola"
            >
              <span className="icon-text">1</span>
            </button>
          )}
          <button
            className={`icon-btn${prefs.manga ? " icon-btn-on" : ""}`}
            onClick={() => update({ manga: !prefs.manga })}
            aria-pressed={prefs.manga}
            aria-label="Leer de derecha a izquierda"
            title="Leer de derecha a izquierda (M)"
          >
            <MangaIcon size={15} />
          </button>
          <button
            className="icon-btn"
            onClick={() => rotate(90)}
            onContextMenu={(e) => {
              e.preventDefault();
              rotate(-90);
            }}
            aria-label="Girar"
            title="Girar 90° (R; con Mayús, al revés)"
          >
            <RotateIcon size={15} />
          </button>
          <button
            className={`icon-btn${prefs.thumbs ? " icon-btn-on" : ""}`}
            onClick={() => update({ thumbs: !prefs.thumbs })}
            aria-pressed={prefs.thumbs}
            aria-label="Miniaturas"
            title="Miniaturas (T)"
          >
            <ThumbsIcon size={15} />
          </button>
          <button
            className="icon-btn"
            onClick={() => setFullscreen(!full)}
            aria-label={full ? "Salir de pantalla completa" : "Pantalla completa"}
            title={full ? "Salir de pantalla completa (F)" : "Pantalla completa (F)"}
          >
            {full ? <ExitFullscreenIcon size={15} /> : <FullscreenIcon size={15} />}
          </button>
        </div>

        {goingTo === null ? (
          <button
            className="reader-counter"
            onClick={() => setGoingTo(String(index + 1))}
            title="Ir a la página (G)"
          >
            {pages.length ? `${index + 1} / ${pages.length}` : ""}
          </button>
        ) : (
          <form onSubmit={submitGoto} className="reader-goto">
            <input
              autoFocus
              value={goingTo}
              onChange={(e) => setGoingTo(e.target.value)}
              onBlur={() => setGoingTo(null)}
              inputMode="numeric"
              aria-label={`Ir a la página, de 1 a ${pages.length}`}
            />
            <span className="reader-goto-total">/ {pages.length}</span>
          </form>
        )}

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
        {!zoomed && (
          <>
            <div
              className="reader-zone reader-zone-left"
              onClick={prefs.manga ? goNext : goPrev}
            />
            <div
              className="reader-zone reader-zone-right"
              onClick={prefs.manga ? goPrev : goNext}
            />
          </>
        )}

        {loading || !layout ? (
          <div className="reader-loading">Cargando…</div>
        ) : (
          <div
            className="reader-frame"
            style={{ width: layout.boxW, height: layout.boxH }}
          >
            <div
              className={`reader-spread${prefs.manga ? " reader-spread-rtl" : ""}`}
              style={{
                width: layout.contentW,
                height: layout.contentH,
                gap: layout.gap,
                transform: `translate(-50%, -50%) rotate(${rotation}deg)`,
              }}
            >
              {imgs.map((img, k) => (
                <img
                  key={pages[shown[k]] ?? k}
                  className="reader-page"
                  src={img.src}
                  alt={`Página ${(shown[k] ?? 0) + 1}`}
                  draggable={false}
                  style={{
                    width: layout.widths[k] * layout.scale,
                    height: layout.contentH,
                  }}
                />
              ))}
            </div>
          </div>
        )}
      </div>

      {prefs.thumbs && pages.length > 0 && (
        <ThumbStrip
          path={comic.path}
          pages={pages}
          current={index}
          onPick={(page) => {
            const at = spreads.findIndex((s) => s.includes(page));
            goToSpread(at < 0 ? 0 : at, "top");
          }}
        />
      )}
    </div>
  );
}
