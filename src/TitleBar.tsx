import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  MinimizeIcon,
  MaximizeIcon,
  RestoreIcon,
  CloseIcon,
} from "./Icons";

const win = getCurrentWindow();

export default function TitleBar({ title }: { title: string }) {
  const [maximized, setMaximized] = useState(false);

  // Windows muestra el icono de "restaurar" en vez del de "maximizar" cuando la
  // ventana ya está maximizada, así que seguimos el estado real de la ventana.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    const sync = () => win.isMaximized().then((v) => !cancelled && setMaximized(v));

    sync();
    win.onResized(sync).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="titlebar-title" data-tauri-drag-region>
        {title}
      </span>
      <div className="titlebar-controls">
        <button
          className="titlebar-btn"
          onClick={() => win.minimize()}
          aria-label="Minimizar"
        >
          <MinimizeIcon />
        </button>
        <button
          className="titlebar-btn"
          onClick={() => win.toggleMaximize()}
          aria-label={maximized ? "Restaurar" : "Maximizar"}
        >
          {maximized ? <RestoreIcon /> : <MaximizeIcon />}
        </button>
        <button
          className="titlebar-btn titlebar-close"
          onClick={() => win.close()}
          aria-label="Cerrar"
        >
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}
