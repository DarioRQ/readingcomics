import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Cuánto se espera antes de pedir un lote, para juntar en una sola petición
 *  todas las miniaturas que entran a la vez al desplazar la tira. */
const BATCH_DELAY = 120;

/**
 * Tira de miniaturas del cómic abierto.
 *
 * Solo se generan las que se ven: un cómic de 200 páginas no puede decodificar
 * 200 imágenes por abrir la tira. Y se piden por lotes, porque cada petición
 * abre el archivo, que en CBR es lo caro.
 */
export default function ThumbStrip({
  path,
  pages,
  current,
  onPick,
}: {
  path: string;
  pages: string[];
  current: number;
  onPick: (page: number) => void;
}) {
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  // El observador no debe reconstruirse cada vez que llega una miniatura, así
  // que lo ya pedido se consulta por referencia y no por estado.
  const known = useRef<Set<string>>(new Set());
  const pending = useRef<Set<string>>(new Set());
  const timer = useRef<number | null>(null);
  const stripRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    known.current = new Set();
    pending.current = new Set();
    setThumbs({});
  }, [path]);

  const request = useCallback(
    (name: string) => {
      if (known.current.has(name)) return;
      known.current.add(name);
      pending.current.add(name);

      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => {
        const names = [...pending.current];
        pending.current.clear();
        if (!names.length) return;
        invoke<Record<string, string>>("get_page_thumbs", { path, names })
          .then((got) => setThumbs((t) => ({ ...t, ...got })))
          .catch(() => {
            // La tira es un extra: si falla, quedan los números y se puede
            // seguir navegando igual.
            names.forEach((n) => known.current.delete(n));
          });
      }, BATCH_DELAY);
    },
    [path],
  );

  useEffect(() => {
    const root = stripRef.current;
    if (!root) return;
    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const name = (entry.target as HTMLElement).dataset.name;
          if (name) request(name);
        }
      },
      { root, rootMargin: "300px 0px" },
    );
    root.querySelectorAll("[data-name]").forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, [pages, request]);

  // Al pasar de página, la tira sigue al lector.
  useEffect(() => {
    const root = stripRef.current;
    root
      ?.querySelector(`[data-page="${current}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "center" });
  }, [current]);

  return (
    <div className="thumb-strip" ref={stripRef}>
      {pages.map((name, i) => (
        <button
          key={name}
          data-name={name}
          data-page={i}
          className={`thumb${i === current ? " thumb-current" : ""}`}
          onClick={() => onPick(i)}
          title={`Página ${i + 1}`}
        >
          {thumbs[name] ? (
            <img src={thumbs[name]} alt="" draggable={false} />
          ) : (
            <span className="thumb-placeholder" />
          )}
          <span className="thumb-number">{i + 1}</span>
        </button>
      ))}
    </div>
  );
}
