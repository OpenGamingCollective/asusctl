pub use sg::Task;

static ENE_APPLY_VAL: u8 = 0x01; // Value for Apply Changes Register
static ENE_SAVE_VAL: u8 = 0xaa;

static ENE_REG_MODE: u32 = 0x8021; // Mode Selection Register
static ENE_REG_SPEED: u32 = 0x8022; // Speed Control Register
static ENE_REG_DIRECTION: u32 = 0x8023; // Direction Control Register

static ENE_REG_APPLY: u32 = 0x80a0;
static _ENE_REG_COLORS_DIRECT_V2: u32 = 0x8100; // to read the colurs
static ENE_REG_COLORS_EFFECT_V2: u32 = 0x8160;

fn data(reg: u32, arg_count: u8) -> [u8; 16] {
    let mut cdb = [0u8; 16];
    cdb[0] = 0xec;
    cdb[1] = 0x41;
    cdb[2] = 0x53;
    cdb[3] = ((reg >> 8) & 0x00ff) as u8;
    cdb[4] = (reg & 0x00ff) as u8;
    cdb[5] = 0x00;
    cdb[6] = 0x00;
    cdb[7] = 0x00;
    cdb[8] = 0x00;
    cdb[9] = 0x00;
    cdb[10] = 0x00;
    cdb[11] = 0x00;
    cdb[12] = 0x00;
    cdb[13] = arg_count; // how many u8 in data packet
    cdb[14] = 0x00;
    cdb[15] = 0x00;
    cdb
}

pub(crate) fn rgb_task(led: u32, rgb: &[u8; 3]) -> Task {
    let mut task = Task::new();
    task.set_cdb(data(led * 3 + ENE_REG_COLORS_EFFECT_V2, 3).as_slice());
    task.set_data(rgb, sg::Direction::ToDevice);
    task
}

/// 0-13
pub(crate) fn mode_task(mode: u8) -> Task {
    let mut task = Task::new();
    task.set_cdb(data(ENE_REG_MODE, 1).as_slice());
    task.set_data(&[mode.min(13)], sg::Direction::ToDevice);
    task
}

/// 0-4, fast to slow
pub(crate) fn speed_task(speed: u8) -> Task {
    let mut task = Task::new();
    task.set_cdb(data(ENE_REG_SPEED, 1).as_slice());
    task.set_data(&[speed.min(4)], sg::Direction::ToDevice);
    task
}

/// 0 = forward, 1 = backward
pub(crate) fn dir_task(mode: u8) -> Task {
    let mut task = Task::new();
    task.set_cdb(data(ENE_REG_DIRECTION, 1).as_slice());
    task.set_data(&[mode.min(1)], sg::Direction::ToDevice);
    task
}

pub(crate) fn apply_task() -> Task {
    let mut task = Task::new();
    task.set_cdb(data(ENE_REG_APPLY, 1).as_slice());
    task.set_data(&[ENE_APPLY_VAL], sg::Direction::ToDevice);
    task
}

pub(crate) fn save_task() -> Task {
    let mut task = Task::new();
    task.set_cdb(data(ENE_REG_APPLY, 1).as_slice());
    task.set_data(&[ENE_SAVE_VAL], sg::Direction::ToDevice);
    task
}
