import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import useEscape from "./hooks/useEscape";

import "./App.css";

function App() {
  useEscape();
  useEffect(() => {
    invoke("init_spotlight_window");
  }, []);

  useEffect(() => { }, []);

  return (
    <div className="container">
      <h1>Tauri MacOS Spotlight App</h1>
      <p>
        Press <kbd>Cmd</kbd>+<kbd>k</kbd> to toggle the spotlight window.
      </p>
    </div>
  );
}

export default App;
