import { initSteam } from "@/utils/steam";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

function App() {
  const [counter, setCounter] = useState<number>(0);

  useEffect(() => {
    initSteam();
  }, []);

  useEffect(() => {
    const handleGlobalKeydown = (e: KeyboardEvent) => {
      if (e.shiftKey && e.key === "Tab") {
        e.preventDefault();
        void invoke("activate_overlay");
        return;
      }

      if (e.key === "F12") {
        e.preventDefault();
        void invoke("trigger_screenshot");
      }
    };

    window.addEventListener("keydown", handleGlobalKeydown);

    return () => {
      window.removeEventListener("keydown", handleGlobalKeydown);
    };
  }, []);

  return (
    <>
      <p>App name</p>
      <p>Value: {counter}</p>
      <button onClick={() => setCounter((prev) => prev + 1)}>
        Increase by 1
      </button>
    </>
  );
}

export default App;
