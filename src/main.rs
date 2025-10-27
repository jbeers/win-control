// ...existing code...
use clap::{Parser, ArgAction};
use serde::Serialize;



#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// List all audio output devices
    #[arg(long, action = ArgAction::SetTrue)]
    list_audio_devices: bool,
    
    #[arg(long, action = ArgAction::SetTrue)]
    list_audio_devices_json: bool,

    /// Set default audio output device by ID
    #[arg(long, value_name = "DEVICE_ID")]
    set_audio_device: Option<String>,
}

#[derive(Serialize)]
struct AudioDevice {
    id: String,
    name: String,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    #[link_name = "CoCreateInstance"]
    fn CoCreateInstanceRaw(
        rclsid: *const windows::core::GUID,
        pUnkOuter: *mut core::ffi::c_void,
        dwClsContext: u32,
        riid: *const windows::core::GUID,
        ppv: *mut *mut core::ffi::c_void,
    ) -> i32;
}

#[cfg(target_os = "windows")]
mod audio {
    use windows::Win32::System::Com::{STGM};
    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Foundation::{PROPERTYKEY, RPC_E_CHANGED_MODE};
    use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED};

    use crate::AudioDevice;

    pub fn set_default_output_by_name(name_match: &str) {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut initialized = false;
            if hr == RPC_E_CHANGED_MODE {
                // Already initialized differently; continue.
            } else if hr.is_ok() {
                initialized = true;
            } else {
                eprintln!("CoInitializeEx failed: 0x{:08X}", hr.0 as u32);
                return;
            }

            let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("CoCreateInstance failed: {e}");
                    if initialized { CoUninitialize(); }
                    return;
                }
            };

            let collection = match enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("EnumAudioEndpoints failed: {e}");
                    if initialized { CoUninitialize(); }
                    return;
                }
            };

            let count = match collection.GetCount() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("GetCount failed: {e}");
                    if initialized { CoUninitialize(); }
                    return;
                }
            };

            let mut found_id = None;
            for i in 0..count {
                let device = match collection.Item(i) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let id = match device.GetId() {
                    Ok(id) => id,
                    Err(_) => continue,
                };

                // Match using the device ID string for now
                let device_id_str = match id.to_string() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if device_id_str.to_lowercase().contains(&name_match.to_lowercase()) {
                    found_id = Some(id);
                    break;
                }
            }

            if let Some(id) = found_id {
                if let Err(e) = set_default_device_ffi(&id) {
                    eprintln!("Failed to set default device: {e}");
                }
            } else {
                eprintln!("No matching device found for: {name_match}");
            }

            if initialized {
                CoUninitialize();
            }
        }
    }

    #[allow(non_snake_case)]
    fn set_default_device_ffi(device_id: &windows::core::PWSTR) -> windows::core::Result<()> {
        #[repr(C)]
        struct IPolicyConfigVtbl {
            pub QueryInterface: usize,
            pub AddRef: usize,
            pub Release: usize,
            pub GetMixFormat: usize,
            pub GetDeviceFormat: usize,
            pub SetDeviceFormat: usize,
            pub GetProcessingPeriod: usize,
            pub SetProcessingPeriod: usize,
            pub GetShareMode: usize,
            pub SetShareMode: usize,
            pub GetPropertyValue: usize,
            pub SetPropertyValue: usize,
            pub SetDefaultEndpoint: usize,
            pub SetEndpointVisibility: usize,
        }
        #[repr(C)]
        struct IPolicyConfig {
            vtbl: *const IPolicyConfigVtbl,
        }
        // Known COM classes/interfaces for PolicyConfig:
        // - Vista Client (commonly works): CLSID {294935CE-F637-4E7C-A41B-AB255460B862}, IID {568B9108-44BF-40B4-9006-86AFE5B5A620}
        // - Alternate (some builds):      CLSID {870AF99C-171D-4F9E-AF0D-E63DF40C2BC9}, IID {F8679F50-850A-41CF-9C72-430F290290C8}
        const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x294935ce_f637_4e7c_a41b_ab255460b862);
        const IID_IPOLICY_CONFIG_VISTA: GUID = GUID::from_u128(0x568b9108_44bf_40b4_9006_86afe5b5a620);
        const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
        const IID_IPOLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

        let mut policy_config: *mut IPolicyConfig = core::ptr::null_mut();
        // Try Vista/Client first
        let mut hr = unsafe {
            super::CoCreateInstanceRaw(
                &CLSID_POLICY_CONFIG_CLIENT,
                core::ptr::null_mut(),
                CLSCTX_ALL.0,
                &IID_IPOLICY_CONFIG_VISTA,
                &mut policy_config as *mut _ as *mut *mut core::ffi::c_void,
            )
        };
        if hr != 0 || policy_config.is_null() {
            // Fallback to alternate CLSID/IID
            policy_config = core::ptr::null_mut();
            hr = unsafe {
                super::CoCreateInstanceRaw(
                    &CLSID_POLICY_CONFIG,
                    core::ptr::null_mut(),
                    CLSCTX_ALL.0,
                    &IID_IPOLICY_CONFIG,
                    &mut policy_config as *mut _ as *mut *mut core::ffi::c_void,
                )
            };
            if hr != 0 || policy_config.is_null() {
                return Err(windows::core::Error::from(windows::core::HRESULT(hr)));
            }
        }

        let set_default_endpoint = unsafe { (*(*policy_config).vtbl).SetDefaultEndpoint };
        let set_default_endpoint: extern "system" fn(*mut IPolicyConfig, PCWSTR, u32) -> i32 = unsafe { core::mem::transmute(set_default_endpoint) };
        let hr = set_default_endpoint(policy_config, PCWSTR(device_id.0), eMultimedia.0 as u32);
        let release = unsafe { (*(*policy_config).vtbl).Release };
        let release: extern "system" fn(*mut IPolicyConfig) -> u32 = unsafe { core::mem::transmute(release) };
        release(policy_config);
        if hr != 0 {
            return Err(windows::core::Error::from(windows::core::HRESULT(hr)));
        }
        Ok(())
    }

    pub fn list_devices() -> Vec<AudioDevice> {
        let mut out = Vec::new();
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut initialized = false;
            if hr == RPC_E_CHANGED_MODE {
                // continue
            } else if hr.is_ok() {
                initialized = true;
            } else {
                eprintln!("CoInitializeEx failed while listing: 0x{:08X}", hr.0 as u32);
                return out;
            }

            let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("CoCreateInstance failed while listing: {e}");
                    if initialized { CoUninitialize(); }
                    return out;
                }
            };

            let collection = match enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("EnumAudioEndpoints failed while listing: {e}");
                    if initialized { CoUninitialize(); }
                    return out;
                }
            };

            let count = match collection.GetCount() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("GetCount failed while listing: {e}");
                    if initialized { CoUninitialize(); }
                    return out;
                }
            };

            for i in 0..count {
                if let Ok(device) = collection.Item(i) {

                    let mut name = "".to_string();
                    let mut idValue = "".to_string();

                    // Open the property store for the device
                    use windows::Win32::System::Com::STGM_READ;
                    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
                    if let Ok(store) = device.OpenPropertyStore(STGM_READ) {                        
                        match store.GetValue(&PKEY_Device_FriendlyName) {
                            Ok(p) => {
                                // Convert PROPVARIANT to String
                                name = unsafe {
                                    p.to_string()
                                };
                            },
                            Err(_) => continue,
                        }                        
                    }
                    if let Ok(id) = device.GetId() {
                        if let Ok(id_str) = id.to_string() {
                            idValue = id_str;
                        }
                    }

                    out.push(AudioDevice { id: idValue, name } );
                }
            }

            if initialized {
                CoUninitialize();
            }
        }
        out
    }

    pub fn set_default_output_by_id_str(device_id_match: &str) {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut initialized = false;
            if hr == RPC_E_CHANGED_MODE {
            } else if hr.is_ok() {
                initialized = true;
            } else {
                eprintln!("CoInitializeEx failed: 0x{:08X}", hr.0 as u32);
                return;
            }

            let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("CoCreateInstance failed: {e}");
                    if initialized { CoUninitialize(); }
                    return;
                }
            };

            let collection = match enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("EnumAudioEndpoints failed: {e}");
                    if initialized { CoUninitialize(); }
                    return;
                }
            };

            let count = match collection.GetCount() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("GetCount failed: {e}");
                    if initialized { CoUninitialize(); }
                    return;
                }
            };

            let mut found = None;
            for i in 0..count {
                if let Ok(device) = collection.Item(i) {
                    if let Ok(id) = device.GetId() {
                        if let Ok(id_str) = id.to_string() {
                            if id_str == device_id_match {
                                found = Some(id);
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(id) = found {
                if let Err(e) = set_default_device_ffi(&id) {
                    eprintln!("Failed to set default device: {e}");
                }
            } else {
                eprintln!("Device ID not found: {device_id_match}");
            }

            if initialized {
                CoUninitialize();
            }
        }
    }
}


fn print_devices() -> () {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let devices = audio::list_devices();
        for device in devices {
            println!("{}: {}", device.id, device.name);
        }
    } 
}

fn print_devices_json() -> () {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let devices = audio::list_devices();
        // Serialize to JSON and print
        match serde_json::to_string_pretty(&devices) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Failed to serialize devices: {}", e),
        }
    } 
}

fn select_device(device_id: &str) -> () {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
    }

    #[cfg(target_os = "windows")]
    {
        audio::set_default_output_by_id_str(device_id);
    }
}

fn main() {

    let cli = Cli::parse();

    #[cfg(target_os = "windows")]
    {
        if cli.list_audio_devices {
            print_devices();
            return;
        }
        else if cli.list_audio_devices_json {
            print_devices_json();
            return;
        }
        else if let Some(device_id) = cli.set_audio_device {
            select_device(&device_id);
            return;
        }
    }
     
}
