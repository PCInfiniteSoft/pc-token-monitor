import { useEffect, useState } from "react";
import { useUsageStore } from "./stores/usageStore";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { OverlayWindow } from "./components/OverlayWindow";
import { FirstRunDialog } from "./components/FirstRunDialog";

export default function App() {
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
