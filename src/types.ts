export interface AppConfig {
  library_root: string | null;
}

export interface ComicMeta {
  path: string;
  name: string;
  cover: string | null;
  page_count: number;
}
