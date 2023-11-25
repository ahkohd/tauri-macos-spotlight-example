import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { open } from '@tauri-apps/api/dialog';
import useEscape from "./hooks/useEscape";

import "./App.css";

const fmtSelectedFiles = (files: string[]) => {
  if (files.length === 0) {
    return "Upload a file";
  }

  if (files.length === 1) {
    return files[0];
  }

  return `${files.length} files selected`;
}

function App() {
  const [selectedFile, setSelectedFile] = useState<string[]>([]);

  useEscape();
  useEffect(() => {
    invoke("init_spotlight_window");
  }, []);

  const pickFile = async () => {
    await invoke("will_open_file_picker")

    const selected = await open({
      multiple: true,
      filters: [{
        name: 'Image',
        extensions: ['png', 'jpeg']
      }]
    });

    await invoke("did_close_file_picker")

    if (!selected) {
      setSelectedFile([])
    } else {
      setSelectedFile(Array.isArray(selected) ? selected : [selected]);
    }
  }

  return (
    <div className="container">
      <h2>Tauri MacOS Spotlight App w/ File upload</h2>
      <p style={{ margin: 0 }}>
        Press <kbd>Cmd</kbd>+<kbd>k</kbd> to toggle the spotlight window,
        <br />
        or press <kbd>Esc</kbd> to hide window.
      </p>
      <br />
      <button className="file-upload" onClick={pickFile} type="button">
        {fmtSelectedFiles(selectedFile)}
      </button>
    </div>
  );
}

export default App;
