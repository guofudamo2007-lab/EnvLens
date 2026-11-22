//! The Windows adapter takes independent snapshots. A blocked source affects only its own check.
use crate::{
    runner::Fault,
    system::{Adapter, Cpu, Host, Memory, Os},
};
use std::{
    io,
    mem::size_of,
    os::windows::io::{FromRawHandle, OwnedHandle},
    ptr,
};
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
    Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_MIRRORING_DRIVER},
    System::{
        Diagnostics::ToolHelp::*,
        SystemInformation::*,
        Threading::{GetActiveProcessorCount, ALL_PROCESSOR_GROUPS},
    },
};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
pub fn collect() -> Host {
    Host {
        os: os(),
        cpu: cpu(),
        memory: memory(),
        adapters: adapters(),
        ollama: ollama_process(),
    }
}
fn wide(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|c| *c == 0).unwrap_or(s.len())])
}
fn os() -> Result<Os, Fault> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")?;
    let build: String = key.get_value("CurrentBuildNumber")?;
    let build_num = build.parse::<u32>().map_err(|_| Fault::Invalid)?;
    let mut name: String = key.get_value("ProductName")?;
    let kind: String = key.get_value("InstallationType").unwrap_or_default();
    // Windows 11 retains "Windows 10" in ProductName on many client installations.
    if kind == "Client" && build_num >= 22000 {
        name = name.replacen("Windows 10", "Windows 11", 1);
    }
    let major: u32 = key.get_value("CurrentMajorVersionNumber")?;
    let minor: u32 = key.get_value("CurrentMinorVersionNumber")?;
    let revision: u32 = key.get_value("UBR")?;
    let mut info = SYSTEM_INFO::default();
    // SAFETY: GetNativeSystemInfo fills the caller-owned SYSTEM_INFO structure.
    let arch = unsafe {
        GetNativeSystemInfo(&mut info);
        info.Anonymous.Anonymous.wProcessorArchitecture
    };
    let arch = match arch {
        PROCESSOR_ARCHITECTURE_AMD64 => "x64",
        PROCESSOR_ARCHITECTURE_ARM64 => "ARM64",
        PROCESSOR_ARCHITECTURE_INTEL => "x86",
        _ => "other",
    };
    Ok(Os {
        name,
        version: format!("{major}.{minor}.{build_num}.{revision}"),
        arch: arch.into(),
    })
}
fn cpu() -> Result<Cpu, Fault> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0")?;
    let name: String = key.get_value("ProcessorNameString")?;
    // SAFETY: the documented all-groups selector requires no pointers.
    let threads = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if threads == 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(Cpu {
        name: name.trim().into(),
        threads,
    })
}
fn memory() -> Result<Memory, Fault> {
    let mut m = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: the size and lifetime of the output structure match the Windows ABI.
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(Memory {
        total_kib: m.ullTotalPhys / 1024,
        free_kib: m.ullAvailPhys / 1024,
    })
}
fn adapters() -> Result<Vec<Adapter>, Fault> {
    let mut adapters = vec![];
    for i in 0..64 {
        let mut d = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        // SAFETY: null requests adapters. d has the required size and remains writable for the call.
        if unsafe { EnumDisplayDevicesW(ptr::null(), i, &mut d, 0) } == 0 {
            break;
        }
        if d.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0 {
            continue;
        }
        let name = wide(&d.DeviceString);
        if name.is_empty() || adapters.iter().any(|a: &Adapter| a.name == name) {
            continue;
        }
        let id = wide(&d.DeviceID).to_ascii_lowercase();
        let key = wide(&d.DeviceKey);
        let wddm_driver = key
            .strip_prefix(r"\Registry\Machine\")
            .and_then(|p| RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(p).ok())
            .and_then(|k| k.get_value("DriverVersion").ok());
        let nvidia = id.contains("ven_10de") || name.to_ascii_lowercase().contains("nvidia");
        adapters.push(Adapter {
            name,
            nvidia,
            wddm_driver,
        });
    }
    if adapters.is_empty() {
        Err(Fault::Invalid)
    } else {
        Ok(adapters)
    }
}
fn ollama_process() -> Result<bool, Fault> {
    // SAFETY: the snapshot handle is owned exactly once and dropped on every return path.
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error().into());
    }
    let _owner = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: handle is valid while _owner is live and entry has the ABI-declared size.
    let mut ready = unsafe { Process32FirstW(handle, &mut entry) };
    while ready != 0 {
        let name = wide(&entry.szExeFile);
        if name.eq_ignore_ascii_case("ollama.exe") || name.eq_ignore_ascii_case("ollama app.exe") {
            return Ok(true);
        }
        ready = unsafe { Process32NextW(handle, &mut entry) };
    }
    let err = io::Error::last_os_error();
    if err.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
        Ok(false)
    } else {
        Err(err.into())
    }
}
