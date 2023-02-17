import { invoke } from "@tauri-apps/api/tauri";
import { useEffect } from "react";

const useEscape = () => {
  const handleEscape = () => invoke("hide_spotlight");

  useEffect(() => {
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, []);
};

export default useEscape;
