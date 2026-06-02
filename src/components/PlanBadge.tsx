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
}

export function PlanBadge({ plan, offline }: Props) {
  const label = offline ? "OFFLINE" : LABELS[plan];
  return (
    <span className="font-mono text-[10px] px-1 py-0.5 rounded bg-[#333] text-white tracking-widest">
      [{label}]
    </span>
  );
}
