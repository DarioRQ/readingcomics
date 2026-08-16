import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  CachedSeries,
  MetronCandidate,
  MetronSeries,
  MetronStatus,
  SeriesInfo,
  SeriesCacheMap,
} from "./types";
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
  const [saved, setSaved] = useState<CachedSeries | null>(null);
  const [canAsk, setCanAsk] = useState(false);
  const [asking, setAsking] = useState(false);
  const [askError, setAskError] = useState<string | null>(null);
  // Candidatas cuando la primera elección no es la buena, y el nombre que el
  // usuario escribe a mano si lo deducido de la carpeta no da con la serie.
  const [candidates, setCandidates] = useState<MetronCandidate[] | null>(null);
  const [manual, setManual] = useState("");

  useEffect(() => {
    let cancelled = false;
    setInfo(null);
    invoke<SeriesInfo>("get_series_info", { root, path })
      .then((r) => !cancelled && setInfo(r))
      .catch(() => {});
    setMetron(null);
    setSaved(null);
    setAskError(null);
    setCandidates(null);
    setManual("");

    // Lo consultado antes se recupera de disco: ni se vuelve a preguntar a
    // Metron ni se gasta cuota por reentrar en la carpeta.
    invoke<SeriesCacheMap>("get_series_cache")
      .then((map) => !cancelled && setSaved(map[path] ?? null))
      .catch(() => {});

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

  /** Guarda en disco la serie elegida, venga de la búsqueda automática o de
   *  una candidata escogida a mano. */
  const remember = (found: MetronSeries, owned: number) => {
    setMetron(found);
    setCandidates(null);

    // Solo se da por definitivamente completa una serie terminada: en una
    // serie en emisión el total de hoy no es el final.
    const finished = (found.status ?? "").toLowerCase().includes("complet");
    const record: CachedSeries = {
      metron_id: found.id,
      name: found.name,
      publisher: found.publisher,
      issue_count: found.issue_count,
      status: found.status,
      complete:
        finished && found.issue_count !== null && owned >= found.issue_count,
      fetched_at: Math.floor(Date.now() / 1000),
    };
    setSaved(record);
    invoke("save_series", { path, series: record }).catch(() => {});
  };

  const askMetron = async () => {
    if (!info?.series) return;
    setAsking(true);
    setAskError(null);
    try {
      // El volumen y el año viajan aparte del nombre: en Metron son campos
      // propios, y son los que distinguen un volumen de otro.
      const found = await invoke<MetronSeries | null>("metron_find_series", {
        name: info.series,
        year: info.year,
        volume: info.volume,
      });
      if (!found) {
        // Lista vacía, no nula: así aparece el campo para buscar a mano, que
        // es la única salida cuando la carpeta no se llama como la serie.
        setAskError("Metron no encuentra esta serie");
        setCandidates([]);
        setManual(info.series);
        return;
      }
      remember(found, info.owned.length);
    } catch (e) {
      setAskError(String(e));
    } finally {
      setAsking(false);
    }
  };

  /** Lista las candidatas: para corregir la elección o para buscar por otro
   *  nombre cuando el de la carpeta no es el que usa Metron. */
  const search = async (name: string, useHints: boolean) => {
    if (!name.trim()) return;
    setAsking(true);
    setAskError(null);
    try {
      const found = await invoke<MetronCandidate[]>("metron_search_series", {
        name,
        year: useHints ? info?.year ?? null : null,
        volume: useHints ? info?.volume ?? null : null,
      });
      setCandidates(found);
      if (found.length === 0) setAskError("Metron no conoce esta serie");
    } catch (e) {
      setAskError(String(e));
    } finally {
      setAsking(false);
    }
  };

  const choose = async (candidate: MetronCandidate) => {
    setAsking(true);
    setAskError(null);
    try {
      const detail = await invoke<MetronSeries>("metron_series_detail", {
        id: candidate.id,
      });
      remember(detail, info?.owned.length ?? 0);
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
  // El total del ComicInfo manda; luego lo guardado en disco; y por último lo
  // que se acabe de consultar.
  const total =
    info.total ?? saved?.issue_count ?? metron?.issue_count ?? null;
  const missing =
    total !== null && info.owned.length > 0
      ? Array.from({ length: total }, (_, i) => i + 1).filter(
          (n) => !info.owned.includes(n),
        )
      : info.missing;
  // `saved.complete` es una marca fija: una serie terminada que ya cuadró no
  // vuelve a depender de una consulta.
  const complete =
    saved?.complete || (missing.length === 0 && total !== null && have >= total);

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

        {saved?.issue_count != null && info.total === null && (
          <span className="series-note">
            Total según Metron{saved.status ? ` · ${saved.status}` : ""}
          </span>
        )}

        {info.guessed && (
          <span className="series-note">Deducido del nombre de los archivos</span>
        )}

        {total === null && !saved && canAsk && candidates === null && (
          <button className="series-ask" onClick={askMetron} disabled={asking}>
            {asking ? "Consultando…" : "Buscar en Metron"}
          </button>
        )}

        {/* Una misma cabecera tiene varios volúmenes, así que la elección
            automática puede no ser la que el usuario tiene. */}
        {canAsk && saved?.metron_id != null && candidates === null && (
          <button
            className="series-ask series-ask-quiet"
            onClick={() => search(info.series ?? "", false)}
            disabled={asking}
          >
            ¿No es esta?
          </button>
        )}

        {askError && <span className="series-note">{askError}</span>}

        {candidates !== null && candidates.length > 0 && (
          <div className="series-candidates">
            {candidates.slice(0, 8).map((c) => (
              <button
                key={c.id}
                className="series-candidate"
                onClick={() => choose(c)}
                disabled={asking}
              >
                <span className="series-candidate-name">{c.name}</span>
                <span className="series-candidate-meta">
                  {[
                    c.volume ? `vol. ${c.volume}` : null,
                    c.issue_count != null ? `${c.issue_count} números` : null,
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </button>
            ))}
            {candidates.length > 8 && (
              <span className="series-note">
                y {candidates.length - 8} más; afina el nombre si no está aquí
              </span>
            )}
          </div>
        )}

        {/* Si el nombre de la carpeta no es el que usa Metron, no hay
            deducción que valga: se escribe a mano. */}
        {canAsk && candidates !== null && (
          <form
            className="series-manual"
            onSubmit={(e) => {
              e.preventDefault();
              search(manual, false);
            }}
          >
            <input
              value={manual}
              onChange={(e) => setManual(e.target.value)}
              placeholder="Buscar con otro nombre"
              aria-label="Buscar en Metron con otro nombre"
            />
            <button
              type="submit"
              className="series-ask"
              disabled={asking || !manual.trim()}
            >
              {asking ? "…" : "Buscar"}
            </button>
          </form>
        )}

        {info.untagged > 0 && (
          <span className="series-note">
            {info.untagged} sin metadatos
          </span>
        )}
      </div>
    </div>
  );
}
