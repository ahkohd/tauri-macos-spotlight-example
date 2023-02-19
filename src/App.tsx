import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import useEscape from "./hooks/useEscape";

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
        Press <kbd>Cmd</kbd>+<kbd>k</kbd> to toggle the spotlight window,
        <br />
        or press <kbd>Esc</kbd> to hide window.
      </p>
    </div>
  );
}

export default App;
