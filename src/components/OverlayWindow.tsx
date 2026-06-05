import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUsageStore } from "../stores/usageStore";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";

export function OverlayWindow() {
  const { frontendState, isOffline } = useUsageStore();
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);

  const usage = frontendState?.usage;
  const plan = frontendState?.config.plan ?? "Unknown";
  const offline = isOffline();

  function toggleAlwaysOnTop() {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    invoke("set_always_on_top", { value: next });
  }

  function minimize() {
    getCurrentWindow().hide().catch(() => {});
  }

  return (
    <div
      data-tauri-drag-region
      className="w-full h-full flex flex-col select-none"
      style={{
        // Transparent window: a dark text shadow keeps the light text and bars
        // legible over both light and dark desktop content behind the overlay.
        textShadow:
          "0 1px 2px rgba(0,0,0,0.95), 0 0 4px rgba(0,0,0,0.8), 0 0 1px rgba(0,0,0,0.9)",
      }}
    >
      {/* Header */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-2 py-1 border-b border-[#1e1e1e]"
      >
        <span className="font-mono text-[10px] text-[#555] tracking-widest pointer-events-none">
          ⬡ PC TOKEN MONITOR
        </span>
        <span className="pointer-events-none">
          <PlanBadge plan={plan} offline={offline} />
        </span>
      </div>

      {/* Usage bars */}
      <div className="flex flex-col gap-1.5 px-2 py-1.5 flex-1">
        {usage ? (
          <>
            <UsageBar
              label="5HR"
              utilization={usage.five_hour.utilization}
              resetsAt={usage.five_hour.resets_at}
              labelColor="#00d4ff"
            />
            <UsageBar
              label="7DAY"
              utilization={usage.seven_day.utilization}
              resetsAt={usage.seven_day.resets_at}
              labelColor="#ffd700"
            />
          </>
        ) : (
          <span className="font-mono text-[10px] text-[#444] text-center py-2">
            connecting...
          </span>
        )}
      </div>

      {/* Footer controls */}
      <div className="flex items-center justify-between px-2 py-1 border-t border-[#1e1e1e]">
        <button
          onClick={toggleAlwaysOnTop}
          className={`font-mono text-[9px] tracking-widest transition-colors ${
            alwaysOnTop ? "text-[#00d4ff]" : "text-[#444]"
          }`}
        >
          [⊤ ALWAYS ON TOP]
        </button>
        <button
          onClick={minimize}
          className="font-mono text-[9px] text-[#444] hover:text-white transition-colors"
        >
          [×]
        </button>
      </div>
    </div>
  );
}
