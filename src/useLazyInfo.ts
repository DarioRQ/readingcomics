import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Cola con límite de concurrencia.
 *
 * Sin esto, una carpeta de 500 cómics dispararía 500 invokes a la vez: el
 * backend se satura descomprimiendo todo en paralelo y el resultado es peor
 * que ir en orden. Un puñado de tareas simultáneas aprovecha los núcleos sin
 * atragantar el proceso.
 */
const MAX_CONCURRENT = 4;

let active = 0;
const queue: (() => void)[] = [];

function acquire(): Promise<void> {
  if (active < MAX_CONCURRENT) {
    active++;
    return Promise.resolve();
  }
  return new Promise((resolve) => queue.push(resolve));
}

function release() {
  const next = queue.shift();
  if (next) next();
  else active--;
}

/** Cachea en memoria lo ya resuelto, para que volver atrás sea instantáneo. */
const memo = new Map<string, unknown>();

/** Vacía la caché de memoria (al cambiar de biblioteca). */
export function clearInfoCache() {
  memo.clear();
}

/**
 * Pide los datos caros de una tarjeta solo cuando entra en pantalla.
 *
 * Devuelve la ref que hay que colgar del elemento observado y el resultado
 * cuando llega.
 */
export function useLazyInfo<T>(
  command: "get_comic_info" | "get_folder_info",
  root: string | null,
  path: string,
) {
  const ref = useRef<HTMLDivElement | null>(null);
  const key = `${command}:${path}`;
  const [data, setData] = useState<T | null>(() => (memo.get(key) as T) ?? null);

  useEffect(() => {
    if (data !== null || !root || !ref.current) return;

    const el = ref.current;
    let cancelled = false;

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries[0]?.isIntersecting) return;
        observer.disconnect();

        acquire().then(async () => {
          if (cancelled) {
            release();
            return;
          }
          try {
            const result = await invoke<T>(command, { root, path });
            memo.set(key, result);
            if (!cancelled) setData(result);
          } catch {
            // El backend ya devuelve los fallos dentro del propio resultado;
            // si aun así revienta la llamada, la tarjeta se queda sin portada
            // en vez de tumbar la rejilla entera.
          } finally {
            release();
          }
        });
      },
      // Empieza a cargar un poco antes de que la tarjeta sea visible, para que
      // al bajar rápido las portadas ya estén ahí.
      { rootMargin: "300px" },
    );

    observer.observe(el);
    return () => {
      cancelled = true;
      observer.disconnect();
    };
  }, [command, root, path, key, data]);

  return { ref, data };
}
