import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ComicMeta } from "./types";

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
  const cache = useRef<Map<string, string>>(new Map());

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

  const goNext = useCallback(
    () => setIndex((i) => Math.min(i + 1, pages.length - 1)),
    [pages.length],
  );
  const goPrev = useCallback(() => setIndex((i) => Math.max(i - 1, 0)), []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight" || e.key === " ") goNext();
      else if (e.key === "ArrowLeft") goPrev();
      else if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [goNext, goPrev, onClose]);

  return (
    <div className="reader">
      <div className="reader-topbar">
        <button className="ghost-btn" onClick={onClose}>
          ← Biblioteca
        </button>
        <span className="reader-title">{comic.name}</span>
        <span className="reader-counter">
          {pages.length ? `${index + 1} / ${pages.length}` : ""}
        </span>
      </div>
      <div className="reader-stage">
        <div className="reader-zone reader-zone-left" onClick={goPrev} />
        <div className="reader-zone reader-zone-right" onClick={goNext} />
        {loading || !image ? (
          <div className="reader-loading">Cargando…</div>
        ) : (
          <img
            className="reader-page"
            src={image}
            alt={`Página ${index + 1}`}
          />
        )}
      </div>
    </div>
  );
}
