import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { MetronSeries, MetronStatus, SeriesInfo } from "./types";
import { CheckCircleIcon, WarningIcon } from "./Icons";

/** Agrupa números sueltos en rangos: [3,4,5,9] -> "3-5, 9". */
function summarize(numbers: number[], limit = 12) {
  const ranges: string[] = [];
  let start = numbers[0];
  let prev = numbers[0];

  for (const n of numbers.slice(1)) {
    if (n === prev + 1) {
      prev = n;
      continue;
    }
    ranges.push(start === prev ? `${start}` : `${start}-${prev}`);
    start = n;
    prev = n;
  }
  if (numbers.length) ranges.push(start === prev ? `${start}` : `${start}-${prev}`);

  if (ranges.length <= limit) return ranges.join(", ");
  return `${ranges.slice(0, limit).join(", ")} y ${ranges.length - limit} más`;
}

export default function SeriesBanner({
  root,
  path,
}: {
  root: string;
  path: string;
}) {
  const [info, setInfo] = useState<SeriesInfo | null>(null);
  const [metron, setMetron] = useState<MetronSeries | null>(null);
  const [canAsk, setCanAsk] = useState(false);
  const [asking, setAsking] = useState(false);
  const [askError, setAskError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setInfo(null);
    invoke<SeriesInfo>("get_series_info", { root, path })
      .then((r) => !cancelled && setInfo(r))
      .catch(() => {});
    setMetron(null);
    setAskError(null);
    return () => {
      cancelled = true;
    };
  }, [root, path]);

  // Solo se ofrece consultar Metron si hay cuenta conectada.
  useEffect(() => {
    invoke<MetronStatus>("metron_status")
      .then((s) => setCanAsk(s.connected))
      .catch(() => {});
  }, []);

  const askMetron = async () => {
    if (!info?.series) return;
    setAsking(true);
    setAskError(null);
    try {
      const found = await invoke<MetronSeries | null>("metron_find_series", {
        name: info.series,
      });
      if (found) setMetron(found);
      else setAskError("Metron no conoce esta serie");
    } catch (e) {
      setAskError(String(e));
    } finally {
      setAsking(false);
    }
  };

  // Se muestra si hay serie y al menos un número, venga de metadatos o
  // deducido del nombre de los ficheros. Antes se exigían metadatos, y eso
  // dejaba sin banner —y sin acceso a Metron— justo a quien más lo necesita.
  if (!info || !info.series || info.owned.length === 0) return null;

  const have = info.owned.length;
  // El total del ComicInfo manda; Metron solo rellena cuando no lo declara.
  const total = info.total ?? metron?.issue_count ?? null;
  const missing =
    total !== null && info.owned.length > 0
      ? Array.from({ length: total }, (_, i) => i + 1).filter(
          (n) => !info.owned.includes(n),
        )
      : info.missing;
  const complete = missing.length === 0 && total !== null && have >= total;

  return (
    <div className={`series-banner${complete ? " series-banner-complete" : ""}`}>
      <div className="series-head">
        <span className="series-icon">
          {complete ? <CheckCircleIcon size={16} /> : <WarningIcon size={16} />}
        </span>
        <span className="series-name">{info.series}</span>
        {info.publisher && (
          <span className="series-publisher">{info.publisher}</span>
        )}
      </div>

      <div className="series-body">
        <span className="series-count">
          {total !== null ? (
            <>
              Tienes <strong>{have}</strong> de <strong>{total}</strong>
            </>
          ) : (
            <>
              Tienes <strong>{have}</strong> números
            </>
          )}
        </span>

        {complete ? (
          <span className="series-ok">Colección completa</span>
        ) : missing.length > 0 ? (
          <span className="series-missing">Faltan: {summarize(missing)}</span>
        ) : total === null ? (
          <span className="series-note">
            La serie no declara cuántos números tiene
          </span>
        ) : null}

        {metron && (
          <span className="series-note">
            Total según Metron{metron.year_began ? ` (${metron.year_began})` : ""}
          </span>
        )}

        {info.guessed && (
          <span className="series-note">Deducido del nombre de los archivos</span>
        )}

        {total === null && !metron && canAsk && (
          <button className="series-ask" onClick={askMetron} disabled={asking}>
            {asking ? "Consultando…" : "Buscar en Metron"}
          </button>
        )}

        {askError && <span className="series-note">{askError}</span>}

        {info.untagged > 0 && (
          <span className="series-note">
            {info.untagged} sin metadatos
          </span>
        )}
      </div>
    </div>
  );
}
