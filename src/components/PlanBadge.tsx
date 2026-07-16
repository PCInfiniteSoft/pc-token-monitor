import type { Plan } from "../types";

const LABELS: Record<Plan, string> = {
  Pro: "PRO",
  Max50: "MAX 50",
  Max200: "MAX 200",
  Unknown: "UNKNOWN",
};

interface Props {
  plan: Plan;
  offline: boolean;
  variant?: "hud" | "glass";
}

export function PlanBadge({ plan, offline, variant = "hud" }: Props) {
  const label = offline ? "OFFLINE" : LABELS[plan];
  if (variant === "glass") {
    return (
      <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded-md tracking-wide bg-black/[0.08] text-black/70 dark:bg-white/15 dark:text-white/80">
        {label}
      </span>
    );
  }
  return (
    <span className="font-mono text-[10px] px-1 py-0.5 rounded bg-[#333] text-white tracking-widest">
      [{label}]
    </span>
  );
}
