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
  const [up, setUp] = useState(false);
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

  /**
   * En la última fila de la rejilla no hay sitio por debajo, así que el menú
   * se abre hacia arriba en vez de empujar el scroll.
   */
  const toggle = () => {
    if (!open && rootRef.current) {
      const { bottom } = rootRef.current.getBoundingClientRect();
      const needed = actions.length * 36 + 16;
      setUp(window.innerHeight - bottom < needed);
    }
    setOpen((o) => !o);
  };

  return (
    <div className="card-menu" ref={rootRef} onClick={(e) => e.stopPropagation()}>
      <button
        className="card-menu-trigger"
        onClick={toggle}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Más opciones"
        title="Más opciones"
      >
        <MoreIcon size={15} />
      </button>

      {open && (
        <div
          className={`card-menu-list${up ? " card-menu-list-up" : ""}`}
          role="menu"
        >
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
