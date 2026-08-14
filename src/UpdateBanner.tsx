import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Status = "idle" | "working" | "error";

export default function UpdateBanner() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState<Status>("idle");

  useEffect(() => {
    check()
      .then(setUpdate)
      .catch(() => {});
  }, []);

  if (!update) return null;

  const install = async () => {
    setStatus("working");
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch {
      setStatus("error");
    }
  };

  return (
    <div className="update-banner">
      <span>Actualización disponible: v{update.version}</span>
      <button
        className="primary-btn"
        onClick={install}
        disabled={status === "working"}
      >
        {status === "working"
          ? "Actualizando…"
          : status === "error"
            ? "Reintentar"
            : "Actualizar y reiniciar"}
      </button>
    </div>
  );
}
