use std::time::Duration;

use xynok_std::{ArcRwLock, TState, state_machine};
#[derive(Default)]
pub struct GpuData
{
    count:                    u32,
    must_exit:                bool,
    memory_leaked:            u32,
    cleared_memory_leak_flag: bool,
}
/// toggle value of this and rerun program to trigged the state machine's memory leak flow branch !!!
/// - `false` -> program will make memory leaked and if over `MAX_MEMORY_LEAK_ALLOWED`, it automatically closes.
/// - `true` -> program will reduce the memory leaked to `MIN_MEMORY_LEAK_THRESHOLD` and continue Update forever.
const LOOP_FOREVER: bool = true;
const DELAY: f32 = 0.1f32;
const MEMORY_LEAK_PER_FRAME: u32 = 20u32;
const CLEAR_MEMORY_LEAK_PER_FRAME: u32 = 2u32; // should be less than leak per frame for test can
// be trigged
const MIN_MEMORY_LEAK_THRESHOLD: u32 = 50u32;
const MAX_MEMORY_LEAK_ALLOWED: u32 = 200u32;

state_machine! {
 machine: GpuController,
 machine_data_type: ArcRwLock<GpuData>,
 state_group: GpuState,
 states: [Init, LoadDevice, InitResource, Update, CloseApp, MemoryIsLeaking, ReduceMemoryLeak, MemoryLeakedIsTooHigh],
 initial_state: Init,
 precheck_states: [MemoryIsLeaking, MemoryLeakedIsTooHigh],
 transitions: [
    Init -> LoadDevice,
    LoadDevice -> InitResource,
    InitResource -> Update,
    Update -> Update,
    MemoryIsLeaking -> ReduceMemoryLeak,
    ReduceMemoryLeak -> Update,
    MemoryLeakedIsTooHigh -> CloseApp,
    ]
}
impl TState<ArcRwLock<GpuData>> for Init
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();

        println!("Gpu Initing ...");
        std::thread::sleep(Duration::from_secs_f32(DELAY));
        val.count += 1;

        val.count >= 5
    }
}
impl TState<ArcRwLock<GpuData>> for LoadDevice
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();

        println!("Gpu Loading Device ...");
        std::thread::sleep(Duration::from_secs_f32(DELAY));
        val.count += 1;

        val.count >= 10
    }
}
impl TState<ArcRwLock<GpuData>> for InitResource
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();

        println!("Gpu Initing Resource ...");
        std::thread::sleep(Duration::from_secs_f32(DELAY));
        val.count += 1;
        val.count >= 15
    }
}
impl TState<ArcRwLock<GpuData>> for Update
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();

        println!("Gpu Updating ... count = {}, memory leaked = {}", val.count, val.memory_leaked);
        std::thread::sleep(Duration::from_secs_f32(DELAY));
        val.count += 1;
        val.memory_leaked += MEMORY_LEAK_PER_FRAME;

        true
    }
}

impl TState<ArcRwLock<GpuData>> for CloseApp
{
    /// alway return false, because the main loop will use val.must_exit to determine should break
    /// or not. And ClosingApp is the end of machine, nothing after it.
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();
        println!("Closing App... count = {}", val.count);
        std::thread::sleep(Duration::from_secs_f32(DELAY));
        val.must_exit = true;

        false
    }
}
impl TState<ArcRwLock<GpuData>> for MemoryIsLeaking
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();

        if val.memory_leaked > MIN_MEMORY_LEAK_THRESHOLD && !val.cleared_memory_leak_flag
        {
            std::thread::sleep(Duration::from_secs_f32(DELAY));
            println!("⚠️ Memory leaking !!! ... {}", val.memory_leaked);
            val.cleared_memory_leak_flag = true;
            return true;
        }
        false
    }
}
impl TState<ArcRwLock<GpuData>> for ReduceMemoryLeak
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let mut val = data.write();

        println!("🧹 clearing memory leaked from {} to {}...", val.memory_leaked, val.memory_leaked - CLEAR_MEMORY_LEAK_PER_FRAME);
        std::thread::sleep(Duration::from_secs_f32(DELAY));
        val.memory_leaked -= CLEAR_MEMORY_LEAK_PER_FRAME;

        if LOOP_FOREVER
        {
            val.cleared_memory_leak_flag = false;
        }
        true
    }
}

impl TState<ArcRwLock<GpuData>> for MemoryLeakedIsTooHigh
{
    fn is_valid(&self, data: &mut ArcRwLock<GpuData>) -> bool
    {
        let val = data.write();

        let result = val.memory_leaked > MAX_MEMORY_LEAK_ALLOWED;

        if result
        {
            println!("⚠️ MEMORY LEAKED IS TOO HIGH !!! OVER MAXIMUM ALLOWED {}", MAX_MEMORY_LEAK_ALLOWED);
            return true;
        }
        false
    }
}

fn main()
{
    let mut gpu = GpuController::new(ArcRwLock::new(GpuData::default()));

    loop
    {
        gpu.update();

        if gpu.data().read().must_exit
        {
            println!("✅ State machine finished. count = {}, memory_leaked = {}", gpu.data().read().count, gpu.data().read().memory_leaked);
            break;
        }
    }
}
