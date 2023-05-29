use log::{info, warn};
use iced_x86::{Formatter, Instruction, NasmFormatter};
use windows::Win32::System::Diagnostics::Debug::CONTEXT;

fn log_exception_context(c: &CONTEXT) {
    info!("CONTEXT");
    info!("RIP: {:#x}", c.Rip);
    info!("RAX: {:#x}", c.Rax);
    info!("RBX: {:#x}", c.Rbx);
    info!("RCX: {:#x}", c.Rcx);
    info!("RDX: {:#x}", c.Rdx);
    info!("R8: {:#x}", c.R8);
    info!("R9: {:#x}", c.R9);
    info!("R10: {:#x}", c.R10);
    info!("R11: {:#x}", c.R11);
    info!("R12: {:#x}", c.R12);
    info!("R13: {:#x}", c.R13);
    info!("R14: {:#x}", c.R14);
    info!("R15: {:#x}", c.R15);
    info!("RBP: {:#x}", c.Rbp);
    info!("RSP: {:#x}", c.Rsp);
    info!("RSP[0]: {:#x}", unsafe { *(c.Rsp as *const usize) });
    info!("RSP[1]: {:#x}", unsafe { *((c.Rsp + 0x8) as *const usize) });
    info!("RSP[2]: {:#x}", unsafe { *((c.Rsp + 0x10) as *const usize) });
    info!("RSP[3]: {:#x}", unsafe { *((c.Rsp + 0x18) as *const usize) });
    info!("RSP[4]: {:#x}", unsafe { *((c.Rsp + 0x20) as *const usize) });
    info!("RDI: {:#x}", c.Rdi);
    info!("RSI: {:#x}", c.Rsi);
    info!("SEG GS: {:#x}", c.SegGs);
}

pub fn log_instruction(instruction: Instruction) {
    let mut formatter = NasmFormatter::new();
    let mut output = String::new();
    if instruction.is_invalid() {
        warn!("Tried logging invalid instruction");
        return;
    }

    output.clear();
    formatter.format(&instruction, &mut output);
    // info!("{:016X} {}", instruction.ip(), output);
}
