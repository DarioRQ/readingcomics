import { useState } from "react";
import TitleBar from "./TitleBar";
import Library from "./Library";
import Reader from "./Reader";
import UpdateBanner from "./UpdateBanner";
import type { ComicMeta } from "./types";
import "./App.css";

function App() {
  const [activeComic, setActiveComic] = useState<ComicMeta | null>(null);

  return (
    <div className="app-shell">
      <TitleBar title="readingcomics" />
      <UpdateBanner />
      <div className="app-content">
        {activeComic ? (
          <Reader comic={activeComic} onClose={() => setActiveComic(null)} />
        ) : (
          <Library onOpenComic={setActiveComic} />
        )}
      </div>
    </div>
  );
}

export default App;
