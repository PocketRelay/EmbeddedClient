use log::debug;
use std::{
    alloc::{alloc, GlobalAlloc, Layout},
    ffi::{c_void, CStr, CString},
    io::Write,
    net::Ipv4Addr,
};
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
        0x0B, 0xD0, // or edx,eax
        0x89, 0x56, 0x04, // mov dword ptr ds:[esi+4],edx
        0xC7, 0x06, 0x01, 0x00, 0x00, 0x00, // mov dword ptr ds:[esi],1
    ],
};

pub unsafe fn hook() {
    // hook_dlc();
    // hook_console();
    hook_host_lookup();
    hook_cert_check();
}

#[no_mangle]
pub unsafe extern "system" fn fake_gethostbyname(name: PCSTR) -> *mut HOSTENT {
    let str_name = CStr::from_ptr(name.cast());
    debug!("Got Host Lookup Request {}", str_name.to_string_lossy());

    let result = gethostbyname(name);

    // We are only targetting gosredirecotr for host redirects
    // forward null responses aswell
    if str_name.to_bytes() != b"gosredirector.ea.com" || result.is_null() {
        return result;
    }

    debug!("Faking redirect target");

    let host = CString::new("gosredirector.ea.com")
        .unwrap()
        .into_raw()
        .cast();

    // Allocate aliases list with single null ptr
    let aliases_layout = Layout::array::<*mut i8>(1).unwrap();
    let h_aliases = alloc(aliases_layout) as *mut *mut i8;
    *h_aliases = std::ptr::null_mut();

    // Allocate aliases list with single null ptr
    let addr_list_layout = Layout::array::<*mut i8>(2).unwrap();
    let h_addr_list = alloc(addr_list_layout) as *mut *mut i8;
    *h_addr_list = std::ptr::null_mut();

    let mut address: Vec<i8> = Vec::with_capacity(21);
    address.extend_from_slice(&[127, 0, 0, 1]);
    address.extend("gosredirector.ea.com\0".chars().map(|value| value as i8));

    *h_addr_list = address.as_mut_ptr();
    *(h_addr_list.add(1)) = std::ptr::null_mut();

    let mut fake_result = HOSTENT {
        h_name: host,
        h_aliases,
        h_addrtype: 2,
        h_length: 4,
        h_addr_list,
    };

    debug_host_ent(*result);

    std::ptr::addr_of_mut!(fake_result)
}

unsafe fn debug_host_ent(result: HOSTENT) {
    let h_name = CStr::from_ptr(result.h_name.cast());
    debug!("Name: {}", h_name.to_string_lossy());
    debug!("Aliases: ");
    let mut alias = result.h_aliases;

    loop {
        let value = *alias;
        if value.is_null() {
            break;
        }

        let value = CString::from_raw(value);
        debug!("- {}", value.to_string_lossy());

        alias = alias.add(1);
    }

    debug!("Type: {}", result.h_addrtype);
    debug!("Length: {}", result.h_length);

    debug!("Addresses:");
    let mut addr = result.h_addr_list;
    loop {
        let mut value = *addr;
        if value.is_null() {
            break;
        }

        let mut bytes = [0u8; 4];

        for byte in bytes.iter_mut() {
            *byte = *value as u8;
            value = value.add(1);
        }

        let ip = Ipv4Addr::from(bytes);

        let value = CString::from_raw(value);

        debug!("- {} {}", ip, value.to_string_lossy());

        addr = addr.add(1);
    }
}

#[test]
fn test() {
    let value = 0x3bfc57f0u32.to_be_bytes();
    println!("{:?}", value);
}

unsafe fn hook_host_lookup() {
    Pattern::apply_with_transform(
        &HOSTNAME_LOOKUP_PATTERN,
        4,
        |addr| {
            // Initial -> f652b0

            // == Obtain the address from the call ????
            // call ???? (Obtain the relative call distance)
            let distance = *(addr.add(1 /* Skip call opcode */) as *const usize);

            // Relative jump -> EEF240 (jump to jmp in thunk table)
            let jmp_address = addr.add(5 /* Skip call opcode + address */ + distance);

            debug!("Address of jump @ {:#016x}", jmp_address as usize);

            // == Address to the final ptr
            // jmp dword ptr ds:[????]
            let address = *(jmp_address.add(2 /* Skip ptr jmp opcode */) as *const usize);

            debug!("Address of dst @ {:#016x}", address);

            // Invalid call at -> 019A4DF1

            address as *const u8
        },
        |addr| {
            // Replace the address with out faker function
            let ptr: *mut usize = addr as *mut usize;

            let last_address = *ptr;
            debug!("Previous address @ {:#016x}", last_address);

            *ptr = fake_gethostbyname as usize;

            debug!("New address @ {:#016x}", fake_gethostbyname as usize);
        },
    );
}

unsafe fn hook_dlc() {
    Pattern::apply_with_transform(
        &DLC_PATTERN,
        2,
        |addr| addr.add(9),
        |addr| {
            fill_bytes(addr, &[0xB0, 0x01]);
        },
    );
}

unsafe fn hook_console() {
    Pattern::apply_with_transform(
        &CONSOLE_PATTERN,
        22,
        |addr| addr.add(5),
        |addr| {
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
