import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";
import type { FrontendState } from "../types";

export function useTauriEvents() {
  const setFrontendState = useUsageStore((s) => s.setFrontendState);

  useEffect(() => {
    invoke<FrontendState>("get_state").then(setFrontendState).catch(console.error);

    const unlisten = listen<FrontendState>("usage-updated", (event) => {
      setFrontendState(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setFrontendState]);
}
