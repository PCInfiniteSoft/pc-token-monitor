import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useUsageStore } from "../stores/usageStore";
import { overlayPalette, type BgTheme } from "../overlayPalette";
import { AI_NAME } from "../constants";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";

export function OverlayWindow() {
  const { frontendState, isOffline } = useUsageStore();

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
  const userName = frontendState?.user_name ?? "—";

  function openSettings() {
    invoke("open_settings").catch(() => {});
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
      {/* Header: title + window controls */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-2 py-1 border-b border-[#1e1e1e]"
      >
        <span
          className="font-mono text-[10px] tracking-widest pointer-events-none"
          style={{ color: "var(--ov-muted)" }}
        >
          ⬡ PC TOKEN MONITOR
        </span>
        <div className="flex items-center gap-2">
          <button
            onClick={openSettings}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="settings"
          >
            ⚙
          </button>
          <button
            onClick={minimize}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="close"
          >
            ×
          </button>
        </div>
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
          <span
            className="font-mono text-[10px] text-center py-2"
            style={{ color: "var(--ov-muted)" }}
          >
            connecting...
          </span>
        )}
      </div>

      {/* Footer: Claude user (left) · AI name + plan (right) */}
      <div className="flex items-center justify-between px-2 py-1 border-t border-[#1e1e1e]">
        <span
          className="font-mono text-[9px] tracking-wide pointer-events-none truncate max-w-[90px]"
          style={{ color: "var(--ov-muted)" }}
        >
          👤 {userName}
        </span>
        <div className="flex items-center gap-1.5 pointer-events-none">
          <span className="font-mono text-[9px] tracking-wide" style={{ color: "var(--ov-muted)" }}>
            {AI_NAME}
          </span>
          <PlanBadge plan={plan} offline={offline} />
        </div>
      </div>
    </div>
  );
}
