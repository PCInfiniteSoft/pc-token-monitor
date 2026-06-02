import { useMemo } from "react";

interface Props {
  label: string;
  utilization: number;
  resetsAt: string;
  labelColor: string;
}

function barColor(pct: number): string {
  if (pct >= 90) return "#ff3232";
  if (pct >= 70) return "#ff8c00";
  return "#00c853";
}

function formatCountdown(resetsAt: string): string {
  const diff = new Date(resetsAt).getTime() - Date.now();
  if (diff <= 0) return "resetting...";
  const totalSecs = Math.floor(diff / 1000);
  const days = Math.floor(totalSecs / 86400);
  const hours = Math.floor((totalSecs % 86400) / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

export function UsageBar({ label, utilization, resetsAt, labelColor }: Props) {
  const pct = Math.min(100, Math.round(utilization * 100));
  const color = barColor(pct);
  const countdown = useMemo(() => formatCountdown(resetsAt), [resetsAt]);
  const atLimit = pct >= 100;

  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex items-center gap-2">
        <span className="font-mono text-xs w-8 shrink-0" style={{ color: labelColor }}>
          {label}
        </span>
        <div
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
          className="flex-1 h-2 bg-[#222] rounded-sm overflow-hidden"
        >
          <div
            className="h-full rounded-sm transition-all duration-500"
            style={{ width: `${pct}%`, backgroundColor: color }}
          />
        </div>
        <span
          className="font-mono text-xs w-8 text-right shrink-0"
          style={{ color: atLimit ? "#ff3232" : "#ffffff" }}
        >
          {pct}%
        </span>
      </div>
      <div className="font-mono text-[9px] text-[#666] pl-10">
        {atLimit ? (
          <span className="text-[#ff3232]">Limit reached</span>
        ) : (
          <>reset in {countdown}</>
        )}
      </div>
    </div>
  );
}
