extern "C" {
    fn ekernel();
}

#[derive(Debug)]
pub struct Sysinfo {
    /// Seconds since boot
    pub uptime: usize,
    /// 1, 5, and 15 minute load averages
    pub loads: [usize; 3],
    /// Total usable main memory size
    pub totalram: usize,
    /// Available memory size
    pub freeram: usize,
    /// Amount of shared memory
    pub sharedram: usize,
    /// Memory used by buffers
    pub bufferram: usize,
    /// Total swap space size
    pub totalswap: usize,
    /// Swap space still available
    pub freeswap: usize,
    /// Number of current processes
    pub procs: u16,
    /// Total high memory size
    pub totalhigh: usize,
    /// Available high memory size
    pub freehigh: usize,
    /// Memory unit size in bytes
    pub mem_unit: u32,
}

impl Sysinfo {
    pub fn new(newuptime: usize, newtotalram: usize, newprocs: usize) -> Self {
        Self {
            uptime: newuptime,
            loads: [0; 3],
            totalram: newtotalram,
            freeram: newtotalram - ekernel as usize,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: newprocs as u16,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 1,
        }
    }
}
