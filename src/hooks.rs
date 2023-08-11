use std::ffi::c_void;
use windows_sys::Win32::{
    Foundation::FALSE,
    System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE},
};

const DLC_STR_MASK: &str = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
const DLC_OP_MASK: &[u8] = &[
    0x8B, 0x11, 0x8B, 0x42, 0x0C, 0x57, 0x56, 0xFF, 0xD0, 0x8B, 0xC3, 0x8B, 0x4D, 0xF4, 0x64, 0x89,
    0x0D, 0x00, 0x00, 0x00, 0x00, 0x59, 0x5F, 0x5E, 0x5B, 0x8B, 0xE5, 0x5D, 0xC2, 0x08, 0x00, 0xCC,
    0xCC, 0xCC, 0x8B, 0x41, 0x04, 0x56, 0x85, 0xC0,
];

const CONSOLE_STR_MASK: &str = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
const CONSOLE_OP_MASK: &[u8] = &[
    0x8B, 0x45, 0x0C, 0xC7, 0x00, 0x01, 0x00, 0x00, 0x00, 0x5D, 0xC2, 0x08, 0x00, 0x8B, 0x4D, 0x0C,
    0xC7, 0x01, 0x01, 0x00, 0x00, 0x00, 0x5D, 0xC2, 0x08, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC,
];

const CERT_CHECK_STR_MASK: &str = "xxxxxxxx";
const CERT_CHECK_OP_MASK: &[u8] = &[0xB8, 0xE4, 0xFF, 0xFF, 0xFF, 0x5B, 0x59, 0xC3];

const HOSTNAME_LOOKUP_STR_MASK: &str = "x????xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
const HOSTNAME_LOOKUP_OP_MASK: &[u8; 53] = &[
    0xE8, 0x8B, 0x9F, 0xF8, 0xFF, // call <JMP.&gethostbyname>
    0x85, 0xC0, // test eax,eax
    0x74, 0x2E, // je me3c.F652E7
    0x8B, 0x48, 0x0C, // mov ecx,dword ptr ds:[eax+C]
    0x8B, 0x01, // mov eax,dword ptr ds:[ecx]
    0x0F, 0xB6, 0x10, // movzx edx,byte ptr ds:[eax]
    0x0F, 0xB6, 0x48, 0x01, // movzx ecx,byte ptr ds:[eax+1]
    0xC1, 0xE2, 0x08, // shl edx,8
    0x0B, 0xD1, // or edx,ecx
    0x0F, 0xB6, 0x48, 0x02, // movzx ecx,byte ptr ds:[eax+2]
    0x0F, 0xB6, 0x40, 0x03, // movzx eax,byte ptr ds:[eax+3]
    0xC1, 0xE2, 0x08, // shl edx,8
    0x0B, 0xD1, // or edx,ecx
    0xC1, 0xE2, 0x08, // shl edx,8
    0x0B, 0xD0, // or edx,eax (REPLACE THIS WITH mov    eax,0x7f000000)
    0x89, 0x56, 0x04, // mov dword ptr ds:[esi+4],edx
    0xC7, 0x06, 0x01, 0x00, 0x00, 0x00, // mov dword ptr ds:[esi],1
];

/// Compares the opcodes after the provided address using the provided
/// opcode and pattern
///
/// # Arguments
/// * addr - The address to start matching from
/// * op_mask - The opcodes to match against
/// * str_mask - The str pattern defining how to match the opcodes
unsafe fn compare_mask(addr: *const u8, op_mask: &[u8], str_mask: &str) -> bool {
    str_mask
        .chars()
        .enumerate()
        .zip(op_mask.iter())
        .all(|((offset, mask), op)| mask == '?' || *addr.add(offset) == *op)
}

/// Attempts to find a matching pattern anywhere between the start and
/// end address
///
/// # Arguments
/// * start - The address to start searching from
/// * end - The address to end searching at
/// * op_mask - The opcodes to match against
/// * str_mask - The str pattern defining how to match the opcodes
unsafe fn find_pattern(
    start: isize,
    end: isize,
    op_mask: &[u8],
    str_mask: &str,
) -> Option<*const u8> {
    (start..=end)
        .map(|addr| addr as *const u8)
        .find(|addr| compare_mask(*addr, op_mask, str_mask))
}

pub unsafe fn hook() {
    // hook_dlc();

    // hook_console();
    hook_host_lookup();
    hook_cert_check();
}

unsafe fn hook_host_lookup() {
    let start_addr: isize = 0x401000;
    let end_addr: isize = 0xFFFFFF;

    // Find the pattern for VerifyCertificate
    let call_addr = find_pattern(
        start_addr,
        end_addr,
        HOSTNAME_LOOKUP_OP_MASK,
        HOSTNAME_LOOKUP_STR_MASK,
    );

    let call_addr = match call_addr {
        Some(value) => value,
        None => {
            println!("Failed to find patch position 1 (HOST)");
            return;
        }
    };

    let call_addr = call_addr.add(39);

    println!(
        "Found patch position 1 (HOST) @ {:#016x}",
        call_addr as usize
    );

    let mut old_protect: PAGE_PROTECTION_FLAGS = 0;
    // Protect the memory region
    if VirtualProtect(
        call_addr as *const c_void,
        5,
        PAGE_READWRITE,
        &mut old_protect,
    ) == FALSE
    {
        println!("Failed to protect memory region while hooking patch position 1 (DLC)");
        return;
    }

    // Replacement opcodes
    let new_ops: [u8; 5] = [0xba, 0x00, 0x00, 0x00, 0x7f];

    // Iterate the opcodes and write them to the ptr
    let mut op_ptr: *mut u8 = call_addr as *mut u8;
    for op in new_ops {
        *op_ptr = op;
        op_ptr = op_ptr.add(1);
    }

    // Unprotect the memory region
    VirtualProtect(call_addr as *const c_void, 5, old_protect, &mut old_protect);
}

unsafe fn hook_dlc() {
    let start_addr: isize = 0x401000;
    let end_addr: isize = 0xE52000;

    // Find the pattern for VerifyCertificate
    let call_addr = find_pattern(start_addr, end_addr, DLC_OP_MASK, DLC_STR_MASK);

    let call_addr = match call_addr {
        Some(value) => value,
        None => {
            println!("Failed to find patch position 1 (DLC)");
            return;
        }
    };

    let call_addr = call_addr.add(9);

    println!(
        "Found patch position 1 (DLC) @ {:#016x}",
        call_addr as usize
    );

    let mut old_protect: PAGE_PROTECTION_FLAGS = 0;
    // Protect the memory region
    if VirtualProtect(
        call_addr as *const c_void,
        2,
        PAGE_READWRITE,
        &mut old_protect,
    ) == FALSE
    {
        println!("Failed to protect memory region while hooking patch position 1 (DLC)");
        return;
    }

    // Replacement opcodes
    let new_ops: [u8; 2] = [0xB0, 0x01];

    // Iterate the opcodes and write them to the ptr
    let mut op_ptr: *mut u8 = call_addr as *mut u8;
    for op in new_ops {
        *op_ptr = op;
        op_ptr = op_ptr.add(1);
    }

    // Unprotect the memory region
    VirtualProtect(call_addr as *const c_void, 2, old_protect, &mut old_protect);
}

unsafe fn hook_console() {
    let start_addr: isize = 0x401000;
    let end_addr: isize = 0xE52000;

    // Find the pattern for VerifyCertificate
    let call_addr = find_pattern(start_addr, end_addr, CONSOLE_OP_MASK, CONSOLE_STR_MASK);

    let call_addr = match call_addr {
        Some(value) => value,
        None => {
            println!("Failed to find patch position 2 (Console)");
            return;
        }
    };

    let call_addr = call_addr.add(9);

    println!(
        "Found patch position 2 (Console) @ {:#016x}",
        call_addr as usize
    );

    let mut old_protect: PAGE_PROTECTION_FLAGS = 0;
    // Protect the memory region
    if VirtualProtect(
        call_addr as *const c_void,
        22,
        PAGE_READWRITE,
        &mut old_protect,
    ) == FALSE
    {
        println!("Failed to protect memory region while hooking patch position 2 (Console)");
        return;
    }

    // Replacement opcodes
    let mut op_ptr: *mut u8 = call_addr.add(5) as *mut u8;
    for op in [0; 4] {
        *op_ptr = op;
        op_ptr = op_ptr.add(1);
    }

    op_ptr = op_ptr.add(13);
    for op in [0; 4] {
        *op_ptr = op;
        op_ptr = op_ptr.add(1);
    }

    // Unprotect the memory region
    VirtualProtect(
        call_addr as *const c_void,
        22,
        old_protect,
        &mut old_protect,
    );
}

unsafe fn hook_cert_check() {
    let start_addr: isize = 0x401000;
    let end_addr: isize = 0xFFFFFF;

    let call_addr = find_pattern(
        start_addr,
        end_addr,
        CERT_CHECK_OP_MASK,
        CERT_CHECK_STR_MASK,
    );

    let call_addr = match call_addr {
        Some(value) => value,
        None => {
            println!("Failed to find patch position 3 (Cert Check)");
            return;
        }
    };

    println!(
        "Found patch position 3 (Cert Check) @ {:#016x}",
        call_addr as usize
    );

    let mut old_protect: PAGE_PROTECTION_FLAGS = 0;
    // Protect the memory region
    if VirtualProtect(
        call_addr as *const c_void,
        8,
        PAGE_READWRITE,
        &mut old_protect,
    ) == FALSE
    {
        println!("Failed to protect memory region while hooking patch position 3 (Cert Check)");
        return;
    }

    // Replacement opcodes
    let mut op_ptr: *mut u8 = call_addr.add(1) as *mut u8;
    for op in [0; 4] {
        *op_ptr = op;
        op_ptr = op_ptr.add(1);
    }

    // Unprotect the memory region
    VirtualProtect(call_addr as *const c_void, 8, old_protect, &mut old_protect);
}
