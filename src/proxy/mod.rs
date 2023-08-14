
use std::arch::global_asm;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

#[no_mangle]
pub static mut ADDR_TABLE: [*const (); 72] = [std::ptr::null(); 72];
pub static mut PROXY_HANDLE: Option<isize> = None;

#[cfg(target_pointer_width = "64")]
global_asm!(include_str!("binkw.x64.S"));
#[cfg(target_pointer_width = "32")]
global_asm!(include_str!("binkw.x86.S"));
    
// # Safety
pub unsafe fn init() {
    let handle = LoadLibraryA("binkw23.dll\0".as_ptr());
    if handle == 0 {
        eprintln!("Failed to load library");
        return;
    }

    PROXY_HANDLE = Some(handle);

    let symbols: [&str; 72] = ["_BinkBufferBlit@12\0","_BinkBufferCheckWinPos@12\0","_BinkBufferClear@8\0","_BinkBufferClose@4\0","_BinkBufferGetDescription@4\0","_BinkBufferGetError@0\0","_BinkBufferLock@4\0","_BinkBufferOpen@16\0","_BinkBufferSetDirectDraw@8\0","_BinkBufferSetHWND@8\0","_BinkBufferSetOffset@12\0","_BinkBufferSetResolution@12\0","_BinkBufferSetScale@12\0","_BinkBufferUnlock@4\0","_BinkCheckCursor@20\0","_BinkClose@4\0","_BinkCloseTrack@4\0","_BinkControlBackgroundIO@8\0","_BinkControlPlatformFeatures@8\0","_BinkCopyToBuffer@28\0","_BinkCopyToBufferRect@44\0","_BinkDDSurfaceType@4\0","_BinkDX8SurfaceType@4\0","_BinkDX9SurfaceType@4\0","_BinkDoFrame@4\0","_BinkDoFrameAsync@12\0","_BinkDoFrameAsyncWait@8\0","_BinkDoFramePlane@8\0","_BinkGetError@0\0","_BinkGetFrameBuffersInfo@8\0","_BinkGetKeyFrame@12\0","_BinkGetPalette@4\0","_BinkGetRealtime@12\0","_BinkGetRects@8\0","_BinkGetSummary@8\0","_BinkGetTrackData@8\0","_BinkGetTrackID@8\0","_BinkGetTrackMaxSize@8\0","_BinkGetTrackType@8\0","_BinkGoto@12\0","_BinkIsSoftwareCursor@8\0","_BinkLogoAddress@0\0","_BinkNextFrame@4\0","_BinkOpen@8\0","_BinkOpenDirectSound@4\0","_BinkOpenMiles@4\0","_BinkOpenTrack@8\0","_BinkOpenWaveOut@4\0","_BinkPause@8\0","_BinkRegisterFrameBuffers@8\0","_BinkRequestStopAsyncThread@4\0","_BinkRestoreCursor@4\0","_BinkService@4\0","_BinkSetError@4\0","_BinkSetFrameRate@8\0","_BinkSetIO@4\0","_BinkSetIOSize@4\0","_BinkSetMemory@8\0","_BinkSetMixBinVolumes@20\0","_BinkSetMixBins@16\0","_BinkSetPan@12\0","_BinkSetSimulate@4\0","_BinkSetSoundOnOff@8\0","_BinkSetSoundSystem@8\0","_BinkSetSoundTrack@8\0","_BinkSetVideoOnOff@8\0","_BinkSetVolume@12\0","_BinkShouldSkip@4\0","_BinkStartAsyncThread@8\0","_BinkWait@4\0","_BinkWaitStopAsyncThread@4\0","_RADTimerRead@0\0"];

    for (symbol, addr) in symbols.iter().zip(ADDR_TABLE.iter_mut()) {
        *addr = GetProcAddress(handle, symbol.as_ptr()).expect("Missing function") as *const ();
    }
}
    
