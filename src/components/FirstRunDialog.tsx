import { invoke } from "@tauri-apps/api/core";
import type { Plan } from "../types";

const PLANS: { value: Plan; label: string; desc: string }[] = [
  { value: "Pro", label: "Pro", desc: "Standard Claude plan" },
  { value: "Max50", label: "Max 50", desc: "5× usage multiplier" },
  { value: "Max200", label: "Max 200", desc: "20× usage multiplier" },
];

interface Props {
  onDone: () => void;
}

export function FirstRunDialog({ onDone }: Props) {
  async function selectPlan(plan: Plan) {
    await invoke("save_plan", { planStr: plan });
    onDone();
  }

  return (
    <div className="fixed inset-0 bg-[#0a0a0a] flex flex-col items-center justify-center gap-4 p-4">
      <p className="font-mono text-[#00d4ff] text-sm tracking-widest">SELECT PLAN</p>
      {PLANS.map((p) => (
        <button
          key={p.value}
          onClick={() => selectPlan(p.value)}
          className="w-full font-mono text-xs text-white bg-[#1a1a1a] hover:bg-[#333] border border-[#333] rounded px-3 py-2 text-left transition-colors"
        >
          <span className="text-[#ffd700]">[{p.label}]</span>{" "}
          <span className="text-[#888]">{p.desc}</span>
        </button>
      ))}
    </div>
  );
}
