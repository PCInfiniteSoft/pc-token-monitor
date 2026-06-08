# Manual Test Cases — pending verification

งาน manual ที่ build/code เสร็จแล้วแต่ยังไม่ verify บนจอจริง. ทำตามขั้น แล้วติ๊ก Pass/Fail.

> Build ปัจจุบัน: release v0.2.0 ที่ `src-tauri/target/release/pc-token-monitor.exe` (build 2026-06-08, icon ใหม่).
> Relaunch: `Start-Process "...\src-tauri\target\release\pc-token-monitor.exe"`

---

## TC-1 — App icon / branding (2026-06-08)

ทำแล้ว: regenerate app icon จาก `Logo/PCTM-app-1024.png` (navy gradient + ∞ + 3 sparkle), แก้ favicon + window title.

| # | ขั้น | คาดหวัง | ผล |
|---|------|---------|-----|
| 1.1 | ดู taskbar ตอน exe รัน | icon = navy tile + วงกลม + ∞ + sparkle 3 สี (ไม่ใช่ Tauri default สีเทา/placeholder) | ☐ |
| 1.2 | Alt+Tab สลับหน้าต่าง | thumbnail แอปโชว์ icon ใหม่ | ☐ |
| 1.3 | เปิด Settings window (จาก overlay) → ดู title bar | title = "PC Token Monitor" + favicon icon ใหม่ (ไม่ใช่ "Tauri + React + Typescript" / vite logo) | ☐ |
| 1.4 | ติดตั้งจาก `bundle/nsis/PC Token Monitor_0.2.0_x64-setup.exe` แล้วดู Start menu + Desktop shortcut | shortcut icon = ใหม่ | ☐ |
| 1.5 | เปิด `%LOCALAPPDATA%\PC Token Monitor\pc-token-monitor.exe` ใน Explorer | file icon (ใน Explorer) = ใหม่ | ☐ |

**Note:** tray icon (เลข % สีพื้น) เป็นคนละตัว — `tray.rs` วาด dynamic ตอน runtime ไม่เกี่ยวกับ app icon นี้. ไม่ต้องเทสในนี้.
**ถ้า icon ยังเก่า:** Windows cache icon — ลอง restart explorer หรือ `ie4uinit.exe -show` เคลียร์ icon cache.

---

## TC-2 — Auto-hide เมื่อไม่มี monitored process (commit b0e66a4, 2026-06-07)

ทำแล้ว: Auto mode → ถ้าไม่มี process ใน allowlist รันอยู่ (PowerShell/Claude/cmd/...) → overlay hide ใน ~1s. enumerate ผ่าน Toolhelp snapshot, throttle ~1s.
Default allowlist: `windowsterminal / powershell / pwsh / claude / cmd / conhost.exe`

**Precondition:** overlay mode = **Auto** (ไม่ใช่ Pinned). เช็ค/ตั้งใน Settings.

| # | ขั้น | คาดหวัง | ผล |
|---|------|---------|-----|
| 2.1 | เปิด PowerShell หรือ Claude Code (process ใน allowlist) | overlay **โชว์** | ☐ |
| 2.2 | โฟกัสไป terminal | overlay pin บนสุด (always-on-top) | ☐ |
| 2.3 | ปิด PowerShell + Claude ทั้งหมด (ไม่เหลือ process ใน allowlist) | overlay **หาย** ภายใน ~1s | ☐ |
| 2.4 | ตอน overlay หาย — คลิก desktop/explorer ไปมา | overlay ยังคงหาย ไม่กลับมา flash | ☐ |
| 2.5 | เปิด PowerShell ใหม่ | overlay **กลับมาโชว์** ภายใน ~1s | ☐ |
| 2.6 | สลับ mode = **Pinned** แล้วปิด terminal หมด | overlay **ยังโชว์** (Pinned ไม่สนใจ monitored process — user ปักเอง) | ☐ |

**Edge cases:**
- 2.7 — ระหว่าง overlay โชว์ แล้วเปิด/ปิด terminal ถี่ ๆ: ไม่ควร flicker รัว (มี throttle ~1s + cache) ☐
- 2.8 — fail-safe: ถ้า Toolhelp snapshot error overlay ควร**โชว์** (assume running) ไม่ใช่หายมั่ว ☐

**ถ้า fail:** ดู log `[aot]` (eprintln). discriminator: overlay ค้าง visible ตอนปิด terminal = `any_monitored_running` คืน true ผิด หรือ foreground ตกไป shell แล้วไม่ recheck.

---

## รันยังไง

1. ปิด instance เก่า: `Stop-Process -Name pc-token-monitor -Force`
2. Relaunch: `Start-Process "C:\Users\PC-Laptop\Documents\Dev Project\PC Token Monitor\src-tauri\target\release\pc-token-monitor.exe"`
3. ทำ TC-1 → TC-2 ติ๊กผล
4. เจอ fail → จดอาการ + log → แจ้ง dev session หน้า
