import { useEffect, useRef, useState } from "react";
import { MoreIcon } from "./Icons";

export interface MenuAction {
  label: string;
  icon: React.ReactNode;
  onSelect: () => void;
}

/**
 * Menú de tres puntos para las tarjetas de la biblioteca.
 *
 * Detiene la propagación de los clics: la tarjeta entera es pulsable, y sin
 * esto abrir el menú abriría también la carpeta.
 */
export default function CardMenu({ actions }: { actions: MenuAction[] }) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="card-menu" ref={rootRef} onClick={(e) => e.stopPropagation()}>
      <button
        className="card-menu-trigger"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Más opciones"
        title="Más opciones"
      >
        <MoreIcon size={15} />
      </button>

      {open && (
        <div className="card-menu-list" role="menu">
          {actions.map((action) => (
            <button
              key={action.label}
              className="card-menu-item"
              role="menuitem"
              onClick={() => {
                setOpen(false);
                action.onSelect();
              }}
            >
              {action.icon}
              {action.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
