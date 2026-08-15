export interface AppConfig {
  library_root: string | null;
  /** Bibliotecas guardadas, para cambiar entre ellas sin buscar la carpeta. */
  libraries: string[];
}

export interface ComicMeta {
  path: string;
  name: string;
}

export interface FolderMeta {
  path: string;
  name: string;
}

/** Metadatos del ComicInfo.xml incrustado en el archivo. */
export interface ComicInfoXml {
  series: string | null;
  number: string | null;
  count: number | null;
  volume: number | null;
  title: string | null;
  publisher: string | null;
  year: number | null;
  writer: string | null;
  summary: string | null;
  language_iso: string | null;
  manga: string | null;
}

/** Datos caros de un cómic, pedidos en diferido. */
export interface ComicInfo {
  cover: string | null;
  page_count: number;
  /** Motivo por el que el cómic no se pudo leer, si es el caso. */
  error: string | null;
  meta: ComicInfoXml | null;
}

/** Estado de una colección detectada en una carpeta. */
export interface SeriesInfo {
  series: string | null;
  publisher: string | null;
  total: number | null;
  owned: number[];
  missing: number[];
  tagged: number;
  untagged: number;
  /** La serie y los números se dedujeron del nombre de los ficheros. */
  guessed: boolean;
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

/** Estado de la conexión con Metron. */
export interface MetronStatus {
  connected: boolean;
  burst_remaining: number | null;
  sustained_remaining: number | null;
}

/** Serie encontrada en Metron. */
export interface MetronSeries {
  id: number;
  name: string;
  year_began: number | null;
  issue_count: number | null;
  publisher: string | null;
  status: string | null;
}

/** Lo que se recuerda de una serie tras consultarla en Metron. */
export interface CachedSeries {
  metron_id: number | null;
  name: string | null;
  publisher: string | null;
  issue_count: number | null;
  /** Estado según Metron: "Completed", "Ongoing"… */
  status: string | null;
  /** Marca fija de colección completa; no requiere volver a consultar. */
  complete: boolean;
  fetched_at: number;
}

export type SeriesCacheMap = Record<string, CachedSeries>;
