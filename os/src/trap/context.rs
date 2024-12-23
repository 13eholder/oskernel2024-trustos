//! Implementation of [`TrapContext`]
use riscv::register::sstatus::{self, Sstatus, SPP};

use crate::signal::{SigSet, SignalStack};

use super::trap_loop;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GeneralRegs {
    pub x: [usize; 32],
}

/// FP registers
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct FloatRegs {
    pub f: [usize; 32],
    pub fcsr: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct MachineContext {
    gp: GeneralRegs,
    fp: FloatRegs,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UserContext {
    pub flags: usize,
    pub link: usize,
    pub stack: SignalStack,
    pub sigmask: SigSet,
    pub __pad: [u8; 128],
    pub mcontext: MachineContext,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
///trap context structure containing sstatus, sepc and registers
pub struct TrapContext {
    /// user trap to kernel need to save those regs
    /// general regs[0..31]
    pub gp: GeneralRegs,
    /// CSR sstatus      
    pub sstatus: Sstatus, // 32
    /// CSR sepc
    pub sepc: usize, // 33
    /// kernel return to user need to save those regs
    pub kernel_sp: usize, // 34
    pub kernel_ra: usize,      // 35 trap_loop
    pub kernel_s: [usize; 12], // 36 - 47
    pub kernel_fp: usize,      // 48
    /// kernel hart address
    pub kernel_tp: usize, // 49
    /// A copy of register a0, useful when we need to restart syscall
    pub origin_a0: usize, // 50
    /// float regs 51-82, fcsr 83
    pub fp: FloatRegs,
}

impl TrapContext {
    ///set stack pointer to x_2 reg (sp)
    pub fn set_sp(&mut self, sp: usize) {
        self.gp.x[2] = sp;
    }
    ///init app context
    pub fn app_init_context(entry: usize, sp: usize, kernel_sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        unsafe {
            sstatus::set_sum();
        }
        let mut cx = Self {
            gp: GeneralRegs { x: [0; 32] },
            sstatus,
            sepc: entry,
            kernel_sp,
            kernel_ra: trap_loop as usize,
            kernel_s: [0; 12],
            kernel_fp: 0,
            kernel_tp: 0,
            origin_a0: 0,
            fp: FloatRegs {
                f: [0; 32],
                fcsr: 0,
            },
        };
        cx.set_sp(sp);
        cx
    }
    pub fn as_mctx(&self) -> MachineContext {
        let mut x = [0; 32];
        x.copy_from_slice(&self.gp.x);
        let mut f = [0; 32];
        f.copy_from_slice(&self.fp.f);
        let fcsr = self.fp.fcsr;
        x[0] = self.sepc; // x0 寄存器永远为0,暂时借用一下,用于保存sepc
        MachineContext {
            gp: GeneralRegs { x },
            fp: FloatRegs { f, fcsr },
        }
    }
    pub fn copy_from_mctx(&mut self, mctx: MachineContext) {
        self.gp = mctx.gp;
        self.fp = mctx.fp;
        self.sepc = self.gp.x[0];
        self.gp.x[0] = 0;
    }
}
