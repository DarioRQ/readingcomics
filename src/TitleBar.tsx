import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

export default function TitleBar({ title }: { title: string }) {
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
          &#8211;
        </button>
        <button
          className="titlebar-btn"
          onClick={() => win.toggleMaximize()}
          aria-label="Maximizar"
        >
          &#9633;
        </button>
        <button
          className="titlebar-btn titlebar-close"
          onClick={() => win.close()}
          aria-label="Cerrar"
        >
          &#10005;
        </button>
      </div>
    </div>
  );
}
