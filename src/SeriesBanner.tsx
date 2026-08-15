import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SeriesInfo } from "./types";
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

  useEffect(() => {
    let cancelled = false;
    setInfo(null);
    invoke<SeriesInfo>("get_series_info", { root, path })
      .then((r) => !cancelled && setInfo(r))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [root, path]);

  // Sin metadatos no hay nada fiable que contar, así que no se muestra nada
  // en vez de inventar una serie a partir del nombre de la carpeta.
  if (!info || !info.series || info.tagged === 0) return null;

  const have = info.owned.length;
  const complete = info.missing.length === 0 && info.total !== null && have >= info.total;

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
          {info.total !== null ? (
            <>
              Tienes <strong>{have}</strong> de <strong>{info.total}</strong>
            </>
          ) : (
            <>
              Tienes <strong>{have}</strong> números
            </>
          )}
        </span>

        {complete ? (
          <span className="series-ok">Colección completa</span>
        ) : info.missing.length > 0 ? (
          <span className="series-missing">
            Faltan: {summarize(info.missing)}
          </span>
        ) : info.total === null ? (
          <span className="series-note">
            La serie no declara cuántos números tiene
          </span>
        ) : null}

        {info.untagged > 0 && (
          <span className="series-note">
            {info.untagged} sin metadatos
          </span>
        )}
      </div>
    </div>
  );
}
