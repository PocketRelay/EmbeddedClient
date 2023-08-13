use std::ffi::{c_void, CStr, CString};
use windows_sys::{
    core::PCSTR,
    Win32::{
        Foundation::FALSE,
        Networking::WinSock::{gethostbyname, HOSTENT},
        System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE},
    },
};

use crate::pattern::{fill_bytes, Pattern};

const DLC_PATTERN: Pattern = Pattern {
    name: "DLC",
    start: 0x401000,
    end: 0xFFFFFF,
    mask: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    op: &[
        0x8B, 0x11, 0x8B, 0x42, 0x0C, 0x57, 0x56, 0xFF, 0xD0, 0x8B, 0xC3, 0x8B, 0x4D, 0xF4, 0x64,
        0x89, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x59, 0x5F, 0x5E, 0x5B, 0x8B, 0xE5, 0x5D, 0xC2, 0x08,
        0x00, 0xCC, 0xCC, 0xCC, 0x8B, 0x41, 0x04, 0x56, 0x85, 0xC0,
    ],
};

const CONSOLE_PATTERN: Pattern = Pattern {
    name: "Console",
    start: 0x401000,
    end: 0xFFFFFF,
    mask: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    op: &[
        0x8B, 0x45, 0x0C, 0xC7, 0x00, 0x01, 0x00, 0x00, 0x00, 0x5D, 0xC2, 0x08, 0x00, 0x8B, 0x4D,
        0x0C, 0xC7, 0x01, 0x01, 0x00, 0x00, 0x00, 0x5D, 0xC2, 0x08, 0x00, 0xCC, 0xCC, 0xCC, 0xCC,
        0xCC,
    ],
};

const VERIFY_CERTIFICATE_PATTERN: Pattern = Pattern {
    name: "VerifyCertificate",
    start: 0x401000,
    end: 0xFFFFFF,
    mask: "xxxxxxxx",
    op: &[0xB8, 0xE4, 0xFF, 0xFF, 0xFF, 0x5B, 0x59, 0xC3],
};

const HOSTNAME_LOOKUP_PATTERN: Pattern = Pattern {
    name: "gethostbyname",
    start: 0x401000,
    end: 0xFFFFFF,
    mask: "x????xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    op: &[
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
    ],
};

pub unsafe fn hook() {
    hook_dlc();
    hook_console();
    hook_host_lookup();
    hook_cert_check();
}

#[no_mangle]
pub unsafe fn fake_gethostbyname(name: PCSTR) -> *mut HOSTENT {
    let str = CStr::from_ptr(name.cast());
    println!("Got Host Lookup Request {}", str.to_string_lossy());
    gethostbyname(name)
}

unsafe fn hook_host_lookup() {
    Pattern::apply_with_transform(
        &HOSTNAME_LOOKUP_PATTERN,
        4,
        |addr| {
            let call_addr: *const isize = addr.add(1).cast();
            let relative_addr = *call_addr;
            addr.add((5 /* opcode + address */ + relative_addr) as usize)
        },
        |addr, start_address| {
            let start_address = start_address as usize;
            let ptr: *mut usize = addr as *mut usize;
            *ptr = (fake_gethostbyname as usize) - (start_address + 5);
        },
    );
}

unsafe fn hook_dlc() {
    Pattern::apply_with_transform(
        &DLC_PATTERN,
        2,
        |addr| addr.add(9),
        |addr, _| {
            fill_bytes(addr, &[0xB0, 0x01]);
        },
    );
}

unsafe fn hook_console() {
    Pattern::apply_with_transform(
        &CONSOLE_PATTERN,
        22,
        |addr| addr.add(5),
        |addr, _| {
            fill_bytes(addr, &[0; 4]);
            fill_bytes(addr.add(18), &[0; 4]);
        },
    );
}

unsafe fn hook_cert_check() {
    Pattern::apply(&VERIFY_CERTIFICATE_PATTERN, 8, |addr| {
        fill_bytes(addr.add(1), &[0; 4]);
    });
}
