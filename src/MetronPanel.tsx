import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { MetronStatus } from "./types";
import { XIcon, CheckCircleIcon } from "./Icons";

const TOKEN_URL = "https://metron.cloud";

export default function MetronPanel({ onClose }: { onClose: () => void }) {
  const [status, setStatus] = useState<MetronStatus | null>(null);
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<MetronStatus>("metron_status").then(setStatus).catch(() => {});
  }, []);

  const connect = async () => {
    setBusy(true);
    setError(null);
    try {
      const s = await invoke<MetronStatus>("metron_connect", { token });
      setStatus(s);
      setToken(""); // no se deja el token dando vueltas en el estado
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async () => {
    setBusy(true);
    try {
      await invoke("metron_disconnect");
      setStatus({ connected: false, burst_remaining: null, sustained_remaining: null });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <h2>Conectar con Metron</h2>
          <button className="icon-btn" onClick={onClose} aria-label="Cerrar">
            <XIcon size={16} />
          </button>
        </div>

        <p className="modal-text">
          Metron es una base de datos de cómics comunitaria y abierta. Sirve para
          completar los datos que no vienen dentro de tus archivos, sobre todo
          cuántos números tiene cada serie.
        </p>

        {status?.connected ? (
          <>
            <div className="metron-connected">
              <CheckCircleIcon size={16} />
              <span>Cuenta conectada</span>
            </div>
            {status.sustained_remaining !== null && (
              <p className="modal-note">
                Te quedan {status.sustained_remaining} consultas hoy.
              </p>
            )}
            <div className="modal-actions">
              <button className="ghost-btn" onClick={disconnect} disabled={busy}>
                Desconectar
              </button>
            </div>
          </>
        ) : (
          <>
            <ol className="modal-steps">
              <li>
                Crea una cuenta gratuita en <code>{TOKEN_URL}</code>
              </li>
              <li>Genera un token de API en la página de tu cuenta</li>
              <li>Pégalo aquí abajo</li>
            </ol>

            <input
              className="modal-input"
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && token && connect()}
              placeholder="Token de API"
              spellCheck={false}
              autoComplete="off"
            />

            {error && <p className="modal-error">{error}</p>}

            <div className="modal-actions">
              <button className="ghost-btn" onClick={onClose}>
                Cancelar
              </button>
              <button
                className="primary-btn"
                onClick={connect}
                disabled={busy || !token.trim()}
              >
                {busy ? "Comprobando…" : "Conectar"}
              </button>
            </div>
          </>
        )}

        <p className="modal-note">
          El token se guarda en el almacén de credenciales del sistema, no en
          ningún fichero de la aplicación. Las consultas solo se hacen cuando las
          pides tú: la app nunca envía nada por su cuenta.
        </p>
      </div>
    </div>
  );
}
