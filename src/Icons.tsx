/**
 * Iconos SVG inline.
 *
 * Se dibujan a mano en vez de usar caracteres Unicode (□, ✕, ←) porque el
 * fallback de fuentes en Windows es inconsistente: los glifos salían con
 * grosores distintos, descentrados o directamente como "tofu".
 *
 * Los de la barra de título usan un viewBox de 10x10 con coordenadas en .5
 * para que el trazo de 1px caiga sobre un píxel exacto y no salga borroso.
 * Los de interfaz usan el estándar de 24x24 con trazo de 2, escalados por CSS.
 */

type IconProps = { size?: number };

/* ---------- Barra de título (trazo fino, estilo Windows) ---------- */

function TitleBarIcon({ children }: { children: React.ReactNode }) {
  return (
    <svg
      width="10"
      height="10"
      viewBox="0 0 10 10"
      fill="none"
      stroke="currentColor"
      strokeWidth="1"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

export const MinimizeIcon = () => (
  <TitleBarIcon>
    <line x1="0" y1="5.5" x2="10" y2="5.5" />
  </TitleBarIcon>
);

export const MaximizeIcon = () => (
  <TitleBarIcon>
    <rect x="0.5" y="0.5" width="9" height="9" />
  </TitleBarIcon>
);

/** Dos cuadrados superpuestos: es lo que Windows muestra cuando ya está maximizada. */
export const RestoreIcon = () => (
  <TitleBarIcon>
    <rect x="0.5" y="2.5" width="7" height="7" />
    <polyline points="2.5,2.5 2.5,0.5 9.5,0.5 9.5,7.5 7.5,7.5" />
  </TitleBarIcon>
);

export const CloseIcon = () => (
  <TitleBarIcon>
    <line x1="0.5" y1="0.5" x2="9.5" y2="9.5" />
    <line x1="9.5" y1="0.5" x2="0.5" y2="9.5" />
  </TitleBarIcon>
);

/* ---------- Interfaz ---------- */

function UiIcon({ size = 18, children }: IconProps & { children: React.ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

export const FolderIcon = (props: IconProps) => (
  <UiIcon {...props}>
    <path d="M3 7a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.7.9l.8 1.2a2 2 0 0 0 1.7.9H19a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  </UiIcon>
);

export const ChevronLeftIcon = (props: IconProps) => (
  <UiIcon {...props}>
    <polyline points="15 18 9 12 15 6" />
  </UiIcon>
);

/** Placeholder cuando un cómic no tiene portada legible. */
export const BookIcon = (props: IconProps) => (
  <UiIcon {...props}>
    <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
    <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
  </UiIcon>
);

/** Carpeta rellena, para la rejilla de la biblioteca. */
export const FolderStackIcon = (props: IconProps) => (
  <UiIcon {...props}>
    <path d="M3 7a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.7.9l.8 1.2a2 2 0 0 0 1.7.9H19a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <path d="M3 11h18" />
  </UiIcon>
);

export const DownloadIcon = (props: IconProps) => (
  <UiIcon {...props}>
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="7 10 12 15 17 10" />
    <line x1="12" y1="15" x2="12" y2="3" />
  </UiIcon>
);
