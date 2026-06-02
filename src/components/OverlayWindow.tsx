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
      className="w-full h-full bg-[#0a0a0a] border border-[#1e1e1e] flex flex-col select-none"
    >
      {/* Header */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-2 py-1 border-b border-[#1e1e1e]"
      >
        <span className="font-mono text-[10px] text-[#555] tracking-widest">
          ⬡ PC TOKEN MONITOR
        </span>
        <PlanBadge plan={plan} offline={offline} />
      </div>

      {/* Usage bars */}
      <div className="flex flex-col gap-2 px-2 py-2 flex-1">
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
