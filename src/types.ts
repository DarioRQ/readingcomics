export interface AppConfig {
  library_root: string | null;
}

export interface ComicMeta {
  path: string;
  name: string;
}

export interface FolderMeta {
  path: string;
  name: string;
}

/** Datos caros de un cómic, pedidos en diferido. */
export interface ComicInfo {
  cover: string | null;
  page_count: number;
  /** Motivo por el que el cómic no se pudo leer, si es el caso. */
  error: string | null;
}

export interface FolderInfo {
  cover: string | null;
  comic_count: number;
}

export interface ReadState {
  read: boolean;
  /** Última página vista, para reanudar donde se dejó. */
  last_page: number;
}

/** Progreso de toda la biblioteca, indexado por ruta del cómic. */
export type ProgressMap = Record<string, ReadState>;

export interface DirListing {
  path: string;
  /** `null` en la raíz: no se puede navegar por encima de la biblioteca. */
  parent: string | null;
  folders: FolderMeta[];
  comics: ComicMeta[];
}
