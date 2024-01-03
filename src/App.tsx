import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import useEscape from "./hooks/useEscape";

import "./App.css";

function App() {
  useEscape();
  useEffect(() => {
    invoke("init_spotlight_window");
  }, []);

  return (
    <div className="container">
      <h2>Tauri MacOS Spotlight App</h2>
      <p style={{ margin: 0 }}>
        Press <kbd>Cmd</kbd>+<kbd>k</kbd> to toggle the spotlight window,
        <br />
        or press <kbd>Esc</kbd> to hide window.
      </p>
      <form style={{ margin: "10px 0" }}>
        <input type="text" name="text" placeholder="Search..." />
      </form>
      <small className="well">
        This <mark>NSWindow</mark> was converted to <mark>NSPanel</mark> at
        runtime.
      </small>
      <br />
    </div>
  );
}

export default App;
