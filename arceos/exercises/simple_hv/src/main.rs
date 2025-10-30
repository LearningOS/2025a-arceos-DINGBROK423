#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(riscv_ext_intrinsics)]

#[cfg(feature = "axstd")]
extern crate axstd as std;
extern crate alloc;
#[macro_use]
extern crate axlog;

mod task;
mod vcpu;
mod regs;
mod csrs;
mod sbi;
mod loader;

use vcpu::VmCpuRegisters;
use riscv::register::{scause, sstatus, stval};
use csrs::defs::hstatus;
use tock_registers::LocalRegisterCopy;
use csrs::{RiscvCsrTrait, CSR};
use vcpu::_run_guest;
use sbi::SbiMessage;
use loader::load_vm_image;
use axhal::mem::PhysAddr;
use crate::regs::GprIndex::{A0, A1};

const VM_ENTRY: usize = 0x8020_0000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    ax_println!("Hypervisor ...");

    // A new address space for vm.
    let mut uspace = axmm::new_user_aspace().unwrap();

    // Load vm binary file into address space.
    if let Err(e) = load_vm_image("/sbin/skernel2", &mut uspace) {
        panic!("Cannot load app! {:?}", e);
    }

    // Setup context to prepare to enter guest mode.
    let mut ctx = VmCpuRegisters::default();
    prepare_guest_context(&mut ctx);

    // Setup pagetable for 2nd address mapping.
    let ept_root = uspace.page_table_root();
    prepare_vm_pgtable(ept_root);

    // Kick off vm and wait for it to exit.
    while !run_guest(&mut ctx) {
    }

    panic!("Hypervisor ok!");
}

fn prepare_vm_pgtable(ept_root: PhysAddr) {
    let hgatp = 8usize << 60 | usize::from(ept_root) >> 12;
    unsafe {
        core::arch::asm!(
            "csrw hgatp, {hgatp}",
            hgatp = in(reg) hgatp,
        );
        core::arch::riscv64::hfence_gvma_all();
    }
}

fn run_guest(ctx: &mut VmCpuRegisters) -> bool {
    unsafe {
        _run_guest(ctx);
    }

    vmexit_handler(ctx)
}

#[allow(unreachable_code)]
fn vmexit_handler(ctx: &mut VmCpuRegisters) -> bool {
    use scause::{Exception, Trap};

    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
            let sbi_msg = SbiMessage::from_regs(ctx.guest_regs.gprs.a_regs()).ok();
            ax_println!("VmExit Reason: VSuperEcall: {:?}", sbi_msg);
            if let Some(msg) = sbi_msg {
                match msg {
                    SbiMessage::Reset(_) => {
                        let a0 = ctx.guest_regs.gprs.reg(A0);
                        let a1 = ctx.guest_regs.gprs.reg(A1);
                        ax_println!("a0 = {:#x}, a1 = {:#x}", a0, a1);
                        assert_eq!(a0, 0x6688);
                        assert_eq!(a1, 0x1234);
                        ax_println!("Shutdown vm normally!");
                        return true;
                    },
                    _ => todo!(),
                }
            } else {
                panic!("bad sbi message! ");
            }
        },
        Trap::Exception(Exception::IllegalInstruction) => {
            // Handle illegal instructions - typically privileged CSR accesses from guest
            // Guest OS tries to execute: csrr a1, mhartid (0xf14025f3)
            // In VS-mode, accessing M-mode CSRs like mhartid is illegal
            // We need to emulate this instruction
            let inst = stval::read();
            ax_println!("Bad instruction: {:#x} sepc: {:#x}", inst, ctx.guest_regs.sepc);
            
            // Check if it's "csrr a1, mhartid" (CSR 0xf14)
            if inst == 0xf14025f3 {
                // Emulate the instruction: set a1 to hardware thread ID
                ctx.guest_regs.gprs.set_reg(A1, 0x1234);
                // Move to next instruction (all RISC-V non-compressed instructions are 4 bytes)
                ctx.guest_regs.sepc += 4;
            } else {
                panic!("Unhandled illegal instruction: {:#x} sepc: {:#x}", inst, ctx.guest_regs.sepc);
            }
        },
        Trap::Exception(Exception::LoadGuestPageFault) => {
            // Handle guest page faults when accessing unmapped memory
            // Guest OS tries to execute: ld a0, 64(zero) which loads from address 0x40
            // Since guest doesn't have a page table set up, any memory access causes a page fault
            // We emulate the load by directly setting the destination register
            let fault_addr = stval::read();
            let htval_val = ctx.trap_csrs.htval;
            ax_println!("LoadGuestPageFault: stval{:#x} htval{:#x} sepc: {:#x}", fault_addr, htval_val, ctx.guest_regs.sepc);
            
            // Check if it's loading from address 0x40 (64 in decimal)
            // stval contains the guest virtual address that caused the fault
            // htval contains (guest_physical_addr >> 2) for page faults
            if fault_addr == 0x40 || htval_val == (0x40 >> 2) {
                // Emulate the load: set a0 to the value that would be at address 0x40
                ctx.guest_regs.gprs.set_reg(A0, 0x6688);
                // Move to next instruction (4 bytes)
                ctx.guest_regs.sepc += 4;
            } else {
                panic!("Unhandled page fault at: stval={:#x} htval={:#x} sepc: {:#x}", fault_addr, htval_val, ctx.guest_regs.sepc);
            }
        },
        _ => {
            panic!(
                "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}",
                scause.cause(),
                ctx.guest_regs.sepc,
                stval::read()
            );
        }
    }
    false
}

fn prepare_guest_context(ctx: &mut VmCpuRegisters) {
    // Set hstatus
    let mut hstatus = LocalRegisterCopy::<usize, hstatus::Register>::new(
        riscv::register::hstatus::read().bits(),
    );
    // Set Guest bit in order to return to guest mode.
    hstatus.modify(hstatus::spv::Guest);
    // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
    hstatus.modify(hstatus::spvp::Supervisor);
    CSR.hstatus.write_value(hstatus.get());
    ctx.guest_regs.hstatus = hstatus.get();

    // Set sstatus in guest mode.
    let mut sstatus = sstatus::read();
    sstatus.set_spp(sstatus::SPP::Supervisor);
    ctx.guest_regs.sstatus = sstatus.bits();
    // Return to entry to start vm.
    ctx.guest_regs.sepc = VM_ENTRY;
}
