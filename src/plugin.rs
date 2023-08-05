use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};
use windows_sys::Win32::{Foundation::GetLastError, System::LibraryLoader::LoadLibraryA};

/// Loads all the plugins from the "ASI" directory
pub fn load() {
    let path = Path::new("ASI");
    if !path.exists() {
        return;
    }

    // Loads the directory entries
    let files = match read_dir(path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Failed to read ASI directory: {}", err);
            return;
        }
    };

    files
        // Filter out error entries
        .filter_map(|entry| entry.ok())
        // Filter to only files
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.is_file()))
        // Load only valid .asi entries
        .for_each(|entry| {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if name.ends_with(".asi") {
                load_plugin(path, name.as_ref())
            }
        });
}

/// Loads a plugin at the provided path
///
/// # Arguments
/// * path - The path to load the plugin from
/// * name - The name of the plugin to load
fn load_plugin(path: PathBuf, name: &str) {
    let mut file_path = path.to_string_lossy().to_string();
    file_path.push('\0');

    println!("Loading plugin: {}", name);

    let handle = unsafe { LoadLibraryA(file_path.as_ptr()) };

    if handle == 0 {
        let err = unsafe { GetLastError() };
        eprintln!("Failed to load plugin: {} {}", name, err);
        return;
    }

    println!("Loaded plugin: {}", name);
}
