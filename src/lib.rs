#![allow(clippy::missing_safety_doc)]

use windows_sys::Win32::System::{
    Console::{AllocConsole, FreeConsole},
    LibraryLoader::FreeLibrary,
    SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
};

pub mod api;
pub mod constants;
pub mod hooks;
pub mod interface;
pub mod pattern;
pub mod plugin;
pub mod proxy;
pub mod servers;

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

            // Spawn UI and prepare task set
            std::thread::spawn(|| {
                // Create tokio async runtime
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime");
                let handle = runtime.handle().clone();

                // Initialize the UI
                interface::init(handle);

                // Block for CTRL+C to keep servers alive when window closes
                let shutdown_signal = tokio::signal::ctrl_c();
                let _ = runtime.block_on(shutdown_signal);
            });
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
