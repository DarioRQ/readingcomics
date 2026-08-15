import { useState } from "react";
import TitleBar from "./TitleBar";
import Library from "./Library";
import Reader from "./Reader";
import UpdateBanner from "./UpdateBanner";
import type { ComicMeta } from "./types";
import "./App.css";

function App() {
  const [activeComic, setActiveComic] = useState<ComicMeta | null>(null);
  // Se incrementa al salir de un cómic para que la biblioteca recargue el
  // progreso y refleje lo que se acaba de leer.
  const [progressVersion, setProgressVersion] = useState(0);

  const closeReader = () => {
    setActiveComic(null);
    setProgressVersion((v) => v + 1);
  };

  return (
    <div className="app-shell">
      <TitleBar title="readingcomics" />
      <UpdateBanner />
      <div className="app-content">
        {/* La biblioteca no se desmonta al leer: así se conserva la carpeta en
            la que estabas y no vuelves a la raíz al cerrar el cómic. */}
        <div className={`pane${activeComic ? " pane-hidden" : ""}`}>
          <Library
            onOpenComic={setActiveComic}
            progressVersion={progressVersion}
          />
        </div>
        {activeComic && <Reader comic={activeComic} onClose={closeReader} />}
      </div>
    </div>
  );
}

export default App;
