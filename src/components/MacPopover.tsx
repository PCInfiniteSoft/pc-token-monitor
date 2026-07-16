import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";
import { PlanBadge } from "./PlanBadge";
import { formatCountdown } from "../usageFormat";

const SYSTEM_FONT =
  "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif";

function bandColor(pct: number): string {
  if (pct >= 90) return "#ff453a";
  if (pct >= 70) return "#ff9f0a";
  return "#0a84ff";
}

function barGradient(pct: number): string {
  if (pct >= 90) return "linear-gradient(90deg,#ff6961,#ff453a)";
  if (pct >= 70) return "linear-gradient(90deg,#ffb340,#ff9f0a)";
  return "linear-gradient(90deg,#4aa3ff,#0a84ff)";
}

function UsageRow({
  label,
  utilization,
  resetsAt,
}: {
  label: string;
  utilization: number;
  resetsAt: string;
}) {
  const pct = Math.min(100, Math.round(utilization * 100));
  return (
    <div className="mb-3.5 last:mb-3">
      <div className="flex items-baseline justify-between mb-1.5">
        <span className="text-[11px] font-semibold tracking-wide uppercase text-black/55 dark:text-white/60">
          {label}
        </span>
        <span
          className="text-[17px] font-bold tabular-nums tracking-tight"
          style={{ color: bandColor(pct) }}
        >
          {pct}%
        </span>
      </div>
      <div className="h-[7px] rounded-full overflow-hidden bg-black/10 dark:bg-white/15">
        <div
          className="h-full rounded-full"
          style={{ width: `${pct}%`, background: barGradient(pct) }}
        />
      </div>
      <div className="text-[11px] mt-1.5 tabular-nums text-black/45 dark:text-white/45">
        reset in {formatCountdown(resetsAt)}
      </div>
    </div>
  );
}

export function MacPopover() {
  const { frontendState, isOffline } = useUsageStore();
  const usage = frontendState?.usage;
  const plan = frontendState?.config.plan ?? "Unknown";
  const userName = frontendState?.user_name ?? "—";
  const offline = isOffline();
  const initial = userName.trim().charAt(0).toUpperCase() || "?";

  return (
    <div
      className="w-full h-full p-1.5 bg-transparent select-none"
      style={{ fontFamily: SYSTEM_FONT }}
    >
      <div className="w-full h-full rounded-xl px-4 pt-3.5 pb-3 flex flex-col bg-white/80 dark:bg-[#282828]/80 backdrop-blur-2xl border-[0.5px] border-black/10 dark:border-white/15 shadow-2xl text-[#1c1c1e] dark:text-[#f2f2f7]">
        {/* header */}
        <div className="flex items-center justify-between mb-3.5">
          <span className="text-[14px] font-semibold tracking-tight">
            Claude Usage
          </span>
          <PlanBadge plan={plan} offline={offline} />
        </div>

        {/* usage windows */}
        <div className="flex-1">
          {usage ? (
            <>
              <UsageRow
                label="5-Hour"
                utilization={usage.five_hour.utilization}
                resetsAt={usage.five_hour.resets_at}
              />
              <UsageRow
                label="7-Day"
                utilization={usage.seven_day.utilization}
                resetsAt={usage.seven_day.resets_at}
              />
            </>
          ) : (
            <div className="text-[12px] text-center py-4 text-black/45 dark:text-white/45">
              connecting…
            </div>
          )}
        </div>

        {/* footer */}
        <div className="flex items-center justify-between pt-3 border-t border-black/10 dark:border-white/15">
          <span className="flex items-center gap-1.5 text-[11.5px] font-medium text-black/55 dark:text-white/60">
            <span
              className="w-[17px] h-[17px] rounded-full inline-flex items-center justify-center text-white text-[9px] font-bold"
              style={{ background: "linear-gradient(135deg,#0a84ff,#7c5aa8)" }}
            >
              {initial}
            </span>
            {userName}
          </span>
          <div className="flex gap-1">
            <button
              onClick={() => invoke("open_settings").catch(() => {})}
              aria-label="settings"
              className="w-[26px] h-[26px] rounded-[7px] inline-flex items-center justify-center text-[12px] bg-black/5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/20 text-black/70 dark:text-white/85 transition-colors"
            >
              ⚙
            </button>
            <button
              onClick={() => invoke("quit_app").catch(() => {})}
              aria-label="quit"
              className="w-[26px] h-[26px] rounded-[7px] inline-flex items-center justify-center text-[12px] bg-black/5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/20 text-black/70 dark:text-white/85 transition-colors"
            >
              ⏻
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
