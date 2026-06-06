import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AotMode, FrontendState } from "../types";

export function Settings() {
  const [mode, setMode] = useState<AotMode>("auto");
  const [allowlist, setAllowlist] = useState<string[]>([]);
  const [newEntry, setNewEntry] = useState("");

  useEffect(() => {
    invoke<FrontendState>("get_state")
      .then((s) => {
        setMode(s.config.aot_mode);
        setAllowlist(s.config.aot_allowlist);
      })
      .catch(console.error);
  }, []);

  function changeMode(next: AotMode) {
    setMode(next);
    invoke("set_aot_mode", { mode: next }).catch(console.error);
  }

  function commitList(next: string[]) {
    setAllowlist(next);
    invoke("set_aot_allowlist", { list: next }).catch(console.error);
  }

  function addEntry() {
    const v = newEntry.trim().toLowerCase();
    if (!v || allowlist.includes(v)) {
      setNewEntry("");
      return;
    }
    commitList([...allowlist, v]);
    setNewEntry("");
  }

  function removeEntry(name: string) {
    commitList(allowlist.filter((a) => a !== name));
  }

  return (
    <div className="min-h-screen bg-[#0a0a0a] text-[#e8e8e8] font-mono text-sm p-4 flex flex-col gap-4">
      <h1 className="text-base font-bold">Settings</h1>

      <section className="flex flex-col gap-2">
        <h2 className="text-xs text-[#888] uppercase tracking-widest">Always on top</h2>
        <div className="flex gap-2">
          <button
            onClick={() => changeMode("auto")}
            className={`px-3 py-1 rounded border ${
              mode === "auto" ? "border-[#00d4ff] text-[#00d4ff]" : "border-[#333] text-[#888]"
            }`}
          >
            AUTO
          </button>
          <button
            onClick={() => changeMode("pinned")}
            className={`px-3 py-1 rounded border ${
              mode === "pinned" ? "border-[#00d4ff] text-[#00d4ff]" : "border-[#333] text-[#888]"
            }`}
          >
            PINNED
          </button>
        </div>
        <p className="text-[11px] text-[#666]">
          AUTO: stay on top only while one of the allowed apps is focused. PINNED: always on top.
        </p>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-xs text-[#888] uppercase tracking-widest">Allowed apps (AUTO)</h2>
        <ul className="flex flex-col gap-1">
          {allowlist.map((name) => (
            <li
              key={name}
              className="flex items-center justify-between bg-[#161616] px-2 py-1 rounded"
            >
              <span>{name}</span>
              <button
                onClick={() => removeEntry(name)}
                className="text-[#888] hover:text-[#ff5555]"
                aria-label={`remove ${name}`}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
        <div className="flex gap-2">
          <input
            value={newEntry}
            onChange={(e) => setNewEntry(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") addEntry();
            }}
            placeholder="e.g. code.exe"
            className="flex-1 bg-[#161616] border border-[#333] rounded px-2 py-1 outline-none"
          />
          <button
            onClick={addEntry}
            className="px-3 py-1 rounded border border-[#333] hover:border-[#00d4ff]"
          >
            Add
          </button>
        </div>
      </section>
    </div>
  );
}
