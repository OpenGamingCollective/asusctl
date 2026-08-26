# GPU Mode Switching Implementation Summary

## Overview
The GPU mode switching system in ASUS Linux uses a sophisticated two-attribute approach combined with deferred GPU settings to safely manage transitions between three GPU modes: **Integrated**, **Ultimate** (Discrete), and **Hybrid** (Optimus).

---

## 1. GUI GPU Mode Switch Implementation

### Location
[rog-control-center/src/ui/setup_gpu.rs](rog-control-center/src/ui/setup_gpu.rs)

### UI Components
- **GPU mode dropdown** in the rog-control-center GUI
- **Three selectable modes:**
  - "Integrated" - iGPU only
  - "Ultimate" - dGPU only (only shown if `gpu_mux_mode` attribute is available)
  - "Hybrid" - Optimus/Dynamic GPU switching
- **Visual indication:** Reboot required notice shown to user

### UI State Management
1. **Initialization** ([setup_gpu.rs#L9-L35](rog-control-center/src/ui/setup_gpu.rs#L9-L35)):
   - Reads `gpu_mux_mode()` attribute availability
   - Reads current `dgpu_disable` value
   - Reads current `gpu_mux_mode` value
   - Populates dropdown choices based on hardware capabilities

2. **Initial Index Calculation** ([setup_gpu.rs#L38-L60](rog-control-center/src/ui/setup_gpu.rs#L38-L60)):
   ```
   If gpu_mux_available:
     - mux value = 0 → "Ultimate" (index 1)
     - mux value = 1 + dgpu = 1 → "Integrated" (index 0)
     - fallback → "Hybrid" (index 2)
   Else (no mux support):
     - dgpu = 1 → "Integrated" (index 0)
     - fallback → "Hybrid" (index 1)
   ```

---

## 2. How the Three GPU Modes Are Handled

### Two-Attribute System
The GUI uses **TWO separate firmware attributes** to achieve three modes:

| Mode | dgpu_disable | gpu_mux_mode | Effect |
|------|--------------|--------------|--------|
| **Integrated** | 1 | 1 | iGPU only (dGPU disabled) |
| **Ultimate** | 0 | 0 | dGPU required (GPU MUX enabled) |
| **Hybrid** | 0 | 1 | Dynamic GPU switching (Optimus) |

### Setting Logic ([setup_gpu.rs#L89-L135](rog-control-center/src/ui/setup_gpu.rs#L89-L135))

#### When GPU MUX is available:
```rust
match index {
    0 => {  // Integrated
        dgpu_disable = 1
        gpu_mux_mode = 1
    }
    1 => {  // Ultimate
        dgpu_disable = 0
        gpu_mux_mode = 0
    }
    2 => {  // Hybrid
        dgpu_disable = 0
        gpu_mux_mode = 1
    }
}
```

#### When GPU MUX is NOT available (fallback mode):
```rust
match index {
    0 => {  // Integrated
        dgpu_disable = 1
    }
    1 => {  // Hybrid
        dgpu_disable = 0
    }
}
```

### Callback Flow
1. User clicks dropdown and selects a mode
2. Toast notification displayed: "GPU mode change scheduled — reboot required for changes to apply."
3. Attributes are written via `set_current_value()`
4. UI refreshes to show written values
5. Changes take effect only **after reboot**

---

## 3. Armoury dgpu_disable Command Implementation

### Firmware Attribute Definition
Located in [rog-platform/src/asus_armoury.rs](rog-platform/src/asus_armoury.rs):

- **Attribute name:** `dgpu_disable`
- **Type:** `FirmwareAttributeType::Gpu`
- **Values:** 
  - 0 = dGPU enabled
  - 1 = dGPU disabled (Integrated GPU mode)

### DBUS Interface
[rog-dbus/src/asus_armoury.rs](rog-dbus/src/asus_armoury.rs):
```rust
#[proxy(interface = "xyz.ljones.AsusArmoury")]
pub trait AsusArmoury {
    #[zbus(property)]
    fn set_current_value(&self, value: i32) -> zbus::Result<()>;
    
    #[zbus(property)]
    fn queued_gpu_value(&self) -> zbus::Result<i32>;
    
    async fn apply_queued_gpu_value(&self) -> zbus::Result<bool>;
}
```

### Daemon Implementation
[asusd/src/asus_armoury.rs](asusd/src/asus_armoury.rs#L407-L465):

**Special GPU Attribute Handling:**

When `set_current_value()` is called for a `FirmwareAttributeType::Gpu` attribute:

```rust
FirmwareAttributeType::Gpu => {
    debug!("Queueing GPU attribute {name} = {value} for delayed apply");
    self.queued_gpu.lock().await.insert(self.name(), value);
    return Ok(());  // Early return - NOT applied immediately
}
```

**Key difference from other attributes:** 
- GPU attributes are **QUEUED**, not applied immediately
- Values stored in `Arc<Mutex<HashMap<FirmwareAttribute, i32>>>`
- Actual firmware write is **DEFERRED** until shutdown

### Query Queued Values
`queued_gpu_value()` property returns:
- The queued value if present
- `-1` if no queued value exists
- Only for GPU attribute types

### Apply on Shutdown
Two functions handle application:

1. **`queued_gpu_value()`** - Retrieve queued value without applying
2. **`apply_queued_gpu_value()`** - Apply the queued value to firmware

---

## 4. CLI Command Interface

### asusctl armoury command
Located in [asusctl/src/main.rs#L1045-L1089](asusctl/src/main.rs#L1045-L1089):

```bash
# List all firmware attributes
asusctl armoury list

# Get specific attribute
asusctl armoury get --property dgpu_disable

# Set attribute value
asusctl armoury set --property dgpu_disable --value 1
asusctl armoury set --property gpu_mux_mode --value 0
```

### CLI Set Logic
```rust
fn handle_armoury_command(cmd: &ArmouryCommand) {
    ArmourySubCommand::Set(s) => {
        for attr in attrs.iter() {
            if attr.name()? == s.property {
                let value = if s.value == -1 { 
                    attr.default_value()? 
                } else { 
                    s.value 
                };
                attr.set_current_value(value)?;  // Queues GPU attributes
                print_firmware_attr(attr)?;
            }
        }
    }
}
```

---

## 5. Key Differences: GUI vs CLI

| Aspect | GUI | CLI |
|--------|-----|-----|
| **2-Attribute Coordination** | ✅ Automatically sets BOTH `dgpu_disable` AND `gpu_mux_mode` together | ❌ No coordination; user must set both manually |
| **Mode Selection** | ✅ Simple dropdown with 3 choices | ❌ Must know exact attribute names and values |
| **Hardware Detection** | ✅ Queries `gpu_mux_mode` availability; adapts choices | ❌ No adaptation; must know what's available |
| **User Feedback** | ✅ Toast notification + UI refresh | ❌ Silent unless error occurs |
| **Reboot Notice** | ✅ Prominent message | ❌ None; user must know changes require reboot |
| **Queued Value Handling** | ✅ Transparent; sets both attributes | ✅ Can set individually, but GPU values are queued |

### CLI Workaround
To switch modes via CLI with GUI-like behavior, set both attributes:
```bash
# Integrated mode
asusctl armoury set --property dgpu_disable --value 1
asusctl armoury set --property gpu_mux_mode --value 1

# Ultimate mode
asusctl armoury set --property dgpu_disable --value 0
asusctl armoury set --property gpu_mux_mode --value 0

# Hybrid mode
asusctl armoury set --property dgpu_disable --value 0
asusctl armoury set --property gpu_mux_mode --value 1
```

---

## 6. Deferred GPU Settings System

### Why Deferred?
GPU firmware writes are considered "risky" during normal operation. They are deferred until system shutdown to:
- Ensure discrete GPU is idle
- Prevent conflicts with active GPU processes
- Ensure safe state transitions

### Deferred Settings Flow

1. **User Action (GUI or CLI)**
   - Calls `set_current_value()` on GPU attribute
   - Value is queued in `queued_gpu` HashMap

2. **Query Queued Value**
   - Application can check `queued_gpu_value()` property
   - Returns `-1` if no queued value

3. **System Shutdown Initiated**
   - `asus-shutdown` service receives PrepareForShutdown signal
   - Calls `apply_queued_gpu_value()` on each queued attribute
   - Waits for discrete GPU to become idle (max 8 seconds)
   - Applies pending GPU settings via firmware write

4. **Post-Apply**
   - Changes persist in firmware
   - Take effect on next boot

### asus-shutdown Service
[asus-shutdown/src/main.rs](asusctl/asus-shutdown/src/main.rs):
- Monitors `logind` `PrepareForShutdown` signal
- Retrieves queued GPU values
- Waits for discrete GPU idle (checks `/proc` for open file descriptors)
- Applies values safely
- Handles GPU busy timeout gracefully

---

## 7. GpuMode Enum

Location: [rog-platform/src/platform.rs#L80-L108](rog-platform/src/platform.rs#L80-L108)

```rust
pub enum GpuMode {
    Optimus = 0,        // Hybrid mode (from gpu_mux_mode)
    Integrated = 1,     // iGPU only (from dgpu_disable)
    Egpu = 2,           // External GPU
    Vfio = 3,           // VFIO passthrough mode
    Ultimate = 4,       // dGPU only (from gpu_mux_mode)
    Error = 254,
    NotSupported = 255,
}
```

### Conversion Methods
```rust
// To firmware attribute values
pub fn to_mux_attr(&self) -> u8 {          // Ultimate(4)→0, others→1
pub fn to_dgpu_attr(&self) -> u8 {         // Integrated(1)→1, others→0
pub fn to_egpu_attr(&self) -> u8 {         // Egpu(2)→1, others→0

// From firmware attribute values
pub fn from_mux(num: u8) -> Self {         // 0→Ultimate, 1→Optimus
pub fn from_dgpu(num: u8) -> Self {        // 1→Integrated, 0→Optimus
pub fn from_egpu(num: u8) -> Self {        // 1→Egpu, 0→Optimus
```

---

## 8. File Locations Reference

### GUI Implementation
- Main UI logic: [rog-control-center/src/ui/setup_gpu.rs](rog-control-center/src/ui/setup_gpu.rs)
- UI data model: [rog-control-center/ui/pages/gpu.slint](rog-control-center/ui/pages/gpu.slint)

### Firmware Attribute System
- Attribute definitions: [rog-platform/src/asus_armoury.rs](rog-platform/src/asus_armoury.rs)
- GpuMode enum: [rog-platform/src/platform.rs](rog-platform/src/platform.rs)
- DBUS proxy: [rog-dbus/src/asus_armoury.rs](rog-dbus/src/asus_armoury.rs)

### Daemon Implementation
- Armoury daemon: [asusd/src/asus_armoury.rs](asusd/src/asus_armoury.rs)
- Shutdown handler: [asus-shutdown/src/main.rs](asusctl/asus-shutdown/src/main.rs)

### CLI Interface
- Armoury command handler: [asusctl/src/main.rs#L1045-L1089](asusctl/src/main.rs#L1045-L1089)

---

## 9. Summary

The GPU mode switching system is elegantly designed using:

1. **Two-attribute model** for hardware flexibility (gpu_mux_mode + dgpu_disable)
2. **GUI abstraction** to hide complexity and coordinate both attributes
3. **Deferred GPU writes** to ensure safe firmware operations
4. **Reboot requirement** as a safety measure to prevent conflicts
5. **Queued value system** allowing safe shutdown-time application
6. **CLI fallback** for advanced users who understand the mapping

The GUI provides a user-friendly three-way mode selection while the underlying system manages the complexity of synchronizing multiple firmware attributes safely during shutdown.
