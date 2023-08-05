use exe::{ExportDirectory, ThunkData, VecPE};
use std::io::Write;
use std::{fs::File, path::Path};

const LIBRARY_DLL: &str = "binkw32.dll";
const RUNTIME_DLL: &str = "binkw23.dll";

fn main() {
    println!("cargo:rerun-if-changed={}", LIBRARY_DLL);

    let image = VecPE::from_disk_file(LIBRARY_DLL).expect("Failed to load binkw32.bak");
    let export_directory = ExportDirectory::parse(&image).expect("Failed to load export directory");

    let mut exports: Vec<&str> = export_directory
        .get_export_map(&image)
        .expect("Failed to get export map")
        .into_iter()
        .filter(|(_, value)| matches!(value, ThunkData::Function(_)))
        .map(|(key, _)| key)
        .collect();

    exports.sort();

    write_asm_x64(&exports);
    write_asm_x86(&exports);
    write_exports(&exports);
    write_proxy(&exports);

    let lib_path = Path::new("src/Exports.def");
    let absolute_path = std::fs::canonicalize(lib_path).unwrap();
    println!(
        "cargo:rustc-cdylib-link-arg=/DEF:{}",
        absolute_path.display()
    );
}

fn write_proxy(exports: &[&str]) {
    let mut out = File::create("src/proxy.rs").expect("Failed to create src/proxy.rs");

    let export_len = exports.len();
    let export_list = exports
        .iter()
        .map(|export| format!("\"{}\\0\"", export))
        .collect::<Vec<String>>()
        .join(",");
    let runtime_dll = RUNTIME_DLL;

    writeln!(
        &mut out,
        r#"
use std::arch::global_asm;
use windows_sys::Win32::System::LibraryLoader::{{GetProcAddress, LoadLibraryA}};

#[no_mangle]
pub static mut ADDR_TABLE: [*const (); {export_len}] = [std::ptr::null(); {export_len}];
pub static mut PROXY_HANDLE: Option<isize> = None;

#[cfg(target_pointer_width = "64")]
global_asm!(include_str!("binkw.x64.S"));
#[cfg(target_pointer_width = "32")]
global_asm!(include_str!("binkw.x86.S"));
    
// # Safety
pub unsafe fn init() {{
    let handle = LoadLibraryA("{runtime_dll}\0".as_ptr());
    if handle == 0 {{
        eprintln!("Failed to load library");
        return;
    }}

    PROXY_HANDLE = Some(handle);

    let symbols: [&str; {export_len}] = [{export_list}];

    for (symbol, addr) in symbols.iter().zip(ADDR_TABLE.iter_mut()) {{
        *addr = GetProcAddress(handle, symbol.as_ptr()).expect("Missing function") as *const ();
    }}
}}
    "#
    )
    .unwrap();
}

fn write_asm_x64(exports: &[&str]) {
    let mut out = File::create("src/binkw.x64.S").expect("Failed to create src/binkw.x64.S");

    exports.iter().for_each(|key| {
        writeln!(&mut out, ".globl {}", key).unwrap();
    });

    writeln!(&mut out).unwrap();

    exports.iter().enumerate().for_each(|(index, key)| {
        writeln!(&mut out, "{}:", key).unwrap();
        writeln!(
            &mut out,
            "    jmp qword ptr [rip + ADDR_TABLE + {} * 8]",
            index
        )
        .unwrap();
    });
}

fn write_asm_x86(exports: &[&str]) {
    let mut out = File::create("src/binkw.x86.S").expect("Failed to create src/binkw.x86.S");

    exports.iter().for_each(|key| {
        writeln!(&mut out, ".globl {}", key).unwrap();
    });

    writeln!(&mut out).unwrap();

    exports.iter().enumerate().for_each(|(index, key)| {
        writeln!(&mut out, "{}:", key).unwrap();

        writeln!(&mut out, "    jmp ds:[ADDR_TABLE + {} * 4]", index).unwrap();
    });
}

fn write_exports(exports: &[&str]) {
    let mut out = File::create("src/Exports.def").expect("Failed to create src/Exports.def");
    writeln!(&mut out, "EXPORTS").unwrap();

    exports.iter().for_each(|key| {
        writeln!(&mut out, "	{}", key).unwrap();
    });

    writeln!(&mut out).unwrap();
}
