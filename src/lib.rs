#![allow(clippy::missing_safety_doc)]

use windows_sys::Win32::System::{
    Console::{AllocConsole, FreeConsole},
    LibraryLoader::FreeLibrary,
    SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
};

pub mod hooks;
pub mod pattern;
pub mod plugin;
pub mod proxy;

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
unsafe extern "system" fn DllMain(dll_module: usize, call_reason: u32, _: *mut ()) -> bool {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            // Allocate a console
            AllocConsole();

            env_logger::builder()
                .filter_level(log::LevelFilter::Debug)
                .init();

            // initialize the proxy
            proxy::init();

            // Handles the DLL being attached to the game
            unsafe { hooks::hook() };

            // Load ASI plugins
            plugin::load();
        }
        DLL_PROCESS_DETACH => {
            // free the proxied library
            if let Some(handle) = proxy::PROXY_HANDLE.take() {
                FreeLibrary(handle);
            }

            // Free the console
            FreeConsole();
        }
        _ => {}
    }

    true
}

#[test]
fn test() {
    // 2130706433
    // 7f000001

    let be = [b'x'; 53];
    println!("{}", std::str::from_utf8(&be).unwrap());
}
