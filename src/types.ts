export interface AppConfig {
  library_root: string | null;
}

export interface ComicMeta {
  path: string;
  name: string;
  cover: string | null;
  page_count: number;
}

export interface FolderMeta {
  path: string;
  name: string;
  cover: string | null;
  comic_count: number;
}

export interface DirListing {
  path: string;
  /** `null` en la raíz: no se puede navegar por encima de la biblioteca. */
  parent: string | null;
  folders: FolderMeta[];
  comics: ComicMeta[];
}
