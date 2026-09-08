# ความคืบหน้าโครงการ (Progress)

อัปเดตล่าสุด: **2026-09-08** · commit `a801b8e` · live: https://rachatarath-wq.github.io/Advance-calculator/

ไฟล์นี้สรุป **สิ่งที่ทำเสร็จแล้ว** และ **สิ่งที่ยังขาด** เพื่อใช้ต่อยอดใน session ถัดไป

---

## ✅ เสร็จแล้ว

### 1. Symbolic Calculus (แกนหลัก)
- Parser แบบ recursive-descent + implicit multiplication (`crates/core/src/parser.rs`)
- Symbolic differentiation (`diff.rs`) และ integration (ตาราง rule + **adaptive Simpson** สำหรับ definite, `integrate.rs`)
- Simplify / render (ASCII + LaTeX), eval, sample (เจาะช่องว่าง NaN ตรง singularity)
- WASM bridge (`crates/wasm`) → `web/src/engine.ts` → React UI

### 2. AC Power (ไฟบ้าน)
- `crates/core/src/ac.rs` — คำนวณ `P = (1/T)∫ v·i dt` แบบ numerical
- แสดง P / Vrms / Irms / S / Q / PF + กราฟ `v(t), i(t), p(t)`
- Panel: `web/src/AcPower.tsx` (โหมด Direct I,φ และโหมด RL)

### 3. Chopper / Phase control (resistive)
- `crates/core/src/chopper.rs` — สับ sine wave ช่วง `[α, β]` ต่อครึ่งคลื่น
- `P = (1/T)∫ v²/R dt`, conduction duty = (β−α)/π
- Panel: `web/src/ChopperPower.tsx` (โหมด resistive และโหมด RL)

### 4. RL Load (มอเตอร์ AC) — ตามโจทย์ "ทำทั้งสองแบบ"
- `crates/core/src/rl.rs`:
  - `rl_ac_power` — steady-state: `Z = R + jωL`, กระแสล้าหลัง φ = atan(ωL/R), `P = Vrms·Irms·cosφ`
  - `rl_chopper` — TRIAC phase control: ยิงที่ α, หา **extinction angle β′** ด้วย bisection, แยก **CCM (α ≤ φ)** vs **DCM (α > φ)**, P ด้วย Simpson บน `[α, β′]`
- 23 Rust tests ผ่าน (รวม 5 ตัว RL)

### 5. หน้าค่ากาแฟ (Coffee)
- `web/src/Coffee.tsx` + `web/src/promptpay.ts`
- PromptPay QR โค้ดจริง (EMVCo payload + CRC-16, เบอร์ `0819459785` → id `0066819459785`) ผ่าน `qrcode` package
- ปุ่มคัดลอกเบอร์ + ลิงก์ Buy Me a Coffee → https://buymeacoffee.com/rachata.rath

### 6. Deploy
- GitHub Pages auto ผ่าน `.github/workflows/deploy.yml` (push `main` → build WASM+Vite → deploy)
- Pages เปิดแบบ `build_type: workflow` แล้ว (ไม่ต้อง enable ซ้ำ)

---

## 🚧 ยังขาด / TODO (ค่อยมาทำต่อ)

### A. โมเดลมอเตอร์จริง (ไม่ใช่แค่ RL equivalent)
- ตอนนี้มอเตอร์ = series `R + jωL` ซึ่งให้กำลัง/กระแส แต่ **ไม่ตอบเรื่องความเร็วรอบ** (slip, torque–speed curve, back-EMF)
- โจทย์ค้างจากบทสนทนา: *"ลดไฟลงแต่ให้มอเตอร์หมุนเท่าเดิม"* → ต้องเพิ่มโมเดล induction motor (หรือ V/f control) ถึงจะจำลองได้ถูก
- แนะนำ: module ใหม่ `crates/core/src/motor.rs` + panel ใหม่

### B. หน้า visualize ความสัมพันธ์ α ↔ กำลัง ↔ ความเร็วมอเตอร์
- ตอนนี้แผง Chopper/RL แสดงค่า แต่ยังไม่มีกราฟ/ตารางที่แสดงว่า **ยิ่งเพิ่ม α ยิ่งลด P** และจุดที่มอเตอร์หยุด (α > φ เข้า DCM)
- แนะนำ: เพิ่ม sweep plot (P, Vrms, PF เทียบกับ α 0–180°) ใน Chopper panel

### C. ภาษาไทย (Localization)
- UI ทั้งหมดยังเป็นอังกฤษ มีแค่หน้า Coffee ที่เป็นไทย
- แนะนำ: string dictionary / i18n (อย่างน้อย labels + hint)

### D. README.md ยังไม่ update
- README ยังอธิบายแค่ symbolic calc — ยังไม่มี AC Power / Chopper / RL / Coffee

### E. Power-electronics เพิ่มเติม (ถ้าจะไปต่อ)
- PWM / inverter (DC chopper, buck/boost), three-phase AC, active/reactive power แยกต่อเฟส

### F. Test coverage เพิ่ม
- เพิ่ม test: เปรียบเทียบ `rl_chopper` CCM กับผล full-sine หลายจุด, ขอบ α = 0°/180°, ค่า L มาก/น้อยสุดขั้ว

---

## 📐 วิธีเพิ่ม feature ใหม่ (pattern เดิม)

```
crates/core/src/<new>.rs     → ฟังก์ชันคำนวณ + struct (derive Serialize)
crates/core/src/lib.rs       → เพิ่ม `pub mod <new>;` + `*_json()` wrapper
crates/wasm/src/lib.rs       → `#[wasm_bindgen]` export
web/src/engine.ts            → interface + client fn (JSON.parse)
web/src/<New>.tsx            → React panel
web/src/App.tsx              → เพิ่ม view + nav tab
```

ค่า default ยึด **ไฟบ้านไทย**: `Vm = 311 V` (220 Vrms), `f = 50 Hz`, มุมเป็นองศา

ทดสอบ: `cargo test` → `cd web && npm run build` → `git push origin main` (deploy อัตโนมัติ)
