import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useUsageStore } from "../stores/usageStore";
import { overlayPalette, type BgTheme } from "../overlayPalette";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";

export function OverlayWindow() {
  const { frontendState, isOffline } = useUsageStore();
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);

  const [bgTheme, setBgTheme] = useState<BgTheme>("dark");

  useEffect(() => {
    const unlisten = listen<string>("bg-theme", (event) => {
      setBgTheme(event.payload === "light" ? "light" : "dark");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const pal = overlayPalette(bgTheme);

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
        textShadow: pal.shadow,
        ["--ov-text" as string]: pal.text,
        ["--ov-muted" as string]: pal.muted,
      }}
    >
      {/* Header */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-2 py-1 border-b border-[#1e1e1e]"
      >
        <span className="font-mono text-[10px] tracking-widest pointer-events-none" style={{ color: "var(--ov-muted)" }}>
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
              labelColor={pal.label5}
            />
            <UsageBar
              label="7DAY"
              utilization={usage.seven_day.utilization}
              resetsAt={usage.seven_day.resets_at}
              labelColor={pal.label7}
            />
          </>
        ) : (
          <span className="font-mono text-[10px] text-center py-2" style={{ color: "var(--ov-muted)" }}>
            connecting...
          </span>
        )}
      </div>

      {/* Footer controls */}
      <div className="flex items-center justify-between px-2 py-1 border-t border-[#1e1e1e]">
        <button
          onClick={toggleAlwaysOnTop}
          className="font-mono text-[9px] tracking-widest transition-colors"
          style={{ color: alwaysOnTop ? pal.label5 : "var(--ov-muted)" }}
        >
          [⊤ ALWAYS ON TOP]
        </button>
        <button
          onClick={minimize}
          className="font-mono text-[9px] transition-opacity hover:opacity-70"
          style={{ color: "var(--ov-muted)" }}
        >
          [×]
        </button>
      </div>
    </div>
  );
}
