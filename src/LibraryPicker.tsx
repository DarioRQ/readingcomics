import { useEffect, useRef, useState } from "react";
import {
  FolderIcon,
  ChevronDownIcon,
  CheckIcon,
  PlusIcon,
  XIcon,
} from "./Icons";

/** Nombre legible de una biblioteca a partir de su ruta. */
function libName(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export default function LibraryPicker({
  current,
  libraries,
  onSelect,
  onAdd,
  onRemove,
}: {
  current: string | null;
  libraries: string[];
  onSelect: (path: string) => void;
  onAdd: () => void;
  onRemove: (path: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);

  // Cerrar al pulsar fuera o con Escape.
  useEffect(() => {
    if (!open) return;

    const onPointerDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };

    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="lib-picker" ref={rootRef}>
      <button
        className="lib-trigger"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="menu"
        aria-expanded={open}
        title={current ?? "Elegir biblioteca"}
      >
        <FolderIcon size={15} />
        <span className="lib-trigger-name">
          {current ? libName(current) : "Elegir biblioteca"}
        </span>
        <ChevronDownIcon size={14} />
      </button>

      {open && (
        <div className="lib-menu" role="menu">
          {libraries.length > 0 && (
            <div className="lib-menu-list">
              {libraries.map((path) => {
                const active = path === current;
                return (
                  <div
                    key={path}
                    className={`lib-item${active ? " lib-item-active" : ""}`}
                  >
                    <button
                      className="lib-item-main"
                      role="menuitem"
                      onClick={() => {
                        setOpen(false);
                        if (!active) onSelect(path);
                      }}
                      title={path}
                    >
                      <span className="lib-item-check">
                        {active && <CheckIcon size={13} />}
                      </span>
                      <span className="lib-item-text">
                        <span className="lib-item-name">{libName(path)}</span>
                        <span className="lib-item-path">{path}</span>
                      </span>
                    </button>
                    <button
                      className="lib-item-remove"
                      onClick={() => onRemove(path)}
                      aria-label={`Quitar ${libName(path)} de la lista`}
                      title="Quitar de la lista (no borra nada del disco)"
                    >
                      <XIcon size={13} />
                    </button>
                  </div>
                );
              })}
            </div>
          )}

          <button
            className="lib-add"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              onAdd();
            }}
          >
            <PlusIcon size={14} />
            Añadir biblioteca…
          </button>
        </div>
      )}
    </div>
  );
}
