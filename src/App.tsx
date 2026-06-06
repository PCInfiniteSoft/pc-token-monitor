import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUsageStore } from "./stores/usageStore";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { OverlayWindow } from "./components/OverlayWindow";
import { FirstRunDialog } from "./components/FirstRunDialog";
import { Settings } from "./components/Settings";

export default function App() {
  if (getCurrentWindow().label === "settings") {
    return <Settings />;
  }
  return <OverlayApp />;
}

function OverlayApp() {
  useTauriEvents();
  const frontendState = useUsageStore((s) => s.frontendState);
  const [showFirstRun, setShowFirstRun] = useState(false);

  useEffect(() => {
    if (frontendState?.config.plan === "Unknown") {
      setShowFirstRun(true);
    }
  }, [frontendState?.config.plan]);

  if (showFirstRun) {
    return <FirstRunDialog onDone={() => setShowFirstRun(false)} />;
  }

  return <OverlayWindow />;
}
