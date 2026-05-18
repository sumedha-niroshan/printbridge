```rust
use anyhow::{bail, Context, Result};
use tracing::{debug, info};

#[cfg(windows)]
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::Foundation::HANDLE,
    Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, EnumPrintersW,
        GetDefaultPrinterW, OpenPrinterW, StartDocPrinterW,
        StartPagePrinter, WritePrinter, DOC_INFO_1W,
        PRINTER_ACCESS_USE, PRINTER_DEFAULTSW,
        PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
        PRINTER_INFO_2W,
    },
};

#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
    pub is_online: bool,
}

/// List all printers available on this Windows machine.
pub fn list_printers() -> Result<Vec<PrinterInfo>> {
    #[cfg(windows)]
    {
        list_printers_windows()
    }

    #[cfg(not(windows))]
    {
        Ok(vec![
            PrinterInfo {
                name: "DEV_STUB_Printer_1".to_string(),
                is_default: true,
                is_online: true,
            },
            PrinterInfo {
                name: "DEV_STUB_Thermal_80mm".to_string(),
                is_default: false,
                is_online: true,
            },
        ])
    }
}

#[cfg(windows)]
fn list_printers_windows() -> Result<Vec<PrinterInfo>> {
    use std::mem::size_of;

    let default_name = get_default_printer_name();

    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    let level: u32 = 2;

    let mut needed: u32 = 0;
    let mut returned: u32 = 0;

    // First call to get required buffer size
    unsafe {
        let _ = EnumPrintersW(
            flags,
            PCWSTR::null(),
            level,
            None,
            &mut needed,
            &mut returned,
        );
    }

    if needed == 0 {
        return Ok(vec![]);
    }

    let mut buf: Vec<u8> = vec![0u8; needed as usize];

    let result = unsafe {
        EnumPrintersW(
            flags,
            PCWSTR::null(),
            level,
            Some(buf.as_mut_slice()),
            &mut needed,
            &mut returned,
        )
    };

    if result.is_err() {
        bail!("EnumPrintersW failed: {:?}", result);
    }

    let mut printers = Vec::new();
    let info_size = size_of::<PRINTER_INFO_2W>();

    for i in 0..returned as usize {
        let info_ptr = unsafe {
            buf.as_ptr().add(i * info_size) as *const PRINTER_INFO_2W
        };

        let info = unsafe { &*info_ptr };

        let name = unsafe {
            info.pPrinterName
                .to_string()
                .unwrap_or_default()
        };

        let is_default = default_name.as_deref() == Some(name.as_str());

        // PRINTER_STATUS_OFFLINE = 0x00000080
        let is_online = (info.Status & 0x00000080) == 0;

        printers.push(PrinterInfo {
            name,
            is_default,
            is_online,
        });
    }

    Ok(printers)
}

#[cfg(windows)]
fn get_default_printer_name() -> Option<String> {
    let mut size: u32 = 256;

    let mut buf: Vec<u16> = vec![0u16; size as usize];

    let pwstr = PWSTR(buf.as_mut_ptr());

    let ok = unsafe {
        GetDefaultPrinterW(pwstr, &mut size)
    };

    if ok.as_bool() {
        Some(
            String::from_utf16_lossy(&buf)
                .trim_end_matches('\0')
                .to_string(),
        )
    } else {
        None
    }
}

/// Send raw bytes directly to a printer (ESC/POS or any raw format).
pub fn print_raw(printer_name: &str, data: &[u8]) -> Result<()> {
    info!(
        "Sending {} bytes to printer '{}'",
        data.len(),
        printer_name
    );

    #[cfg(windows)]
    {
        print_raw_windows(printer_name, data)
    }

    #[cfg(not(windows))]
    {
        debug!(
            "DEV STUB: Would print {} bytes to '{}'",
            data.len(),
            printer_name
        );

        Ok(())
    }
}

#[cfg(windows)]
fn print_raw_windows(printer_name: &str, data: &[u8]) -> Result<()> {
    use std::ptr;

    // Convert printer name to null-terminated UTF-16
    let printer_name_wide: Vec<u16> = printer_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut handle = HANDLE::default();

    let defaults = PRINTER_DEFAULTSW {
        pDatatype: PWSTR::null(),
        pDevMode: ptr::null_mut(),
        DesiredAccess: PRINTER_ACCESS_USE,
    };

    unsafe {
        OpenPrinterW(
            PCWSTR(printer_name_wide.as_ptr()),
            &mut handle,
            Some(&defaults as *const _),
        )
        .context("OpenPrinterW failed")?;
    }

    if handle.is_invalid() {
        bail!("Failed to open printer '{}'", printer_name);
    }

    let mut doc_name: Vec<u16> =
        "PrintBridge Job\0".encode_utf16().collect();

    let mut datatype: Vec<u16> =
        "RAW\0".encode_utf16().collect();

    let doc_info = DOC_INFO_1W {
        pDocName: PWSTR(doc_name.as_mut_ptr()),
        pOutputFile: PWSTR::null(),
        pDatatype: PWSTR(datatype.as_mut_ptr()),
    };

    let result: Result<()> = (|| {
        unsafe {
            let job_id = StartDocPrinterW(
                handle,
                1,
                &doc_info as *const _ as *const _,
            );

            if job_id == 0 {
                bail!("StartDocPrinterW failed — job id 0");
            }

            StartPagePrinter(handle)
                .ok()
                .context("StartPagePrinter failed")?;

            let mut written: u32 = 0;

            WritePrinter(
                handle,
                data.as_ptr() as *const _,
                data.len() as u32,
                &mut written,
            )
            .ok()
            .context("WritePrinter failed")?;

            debug!(
                "Wrote {}/{} bytes to printer",
                written,
                data.len()
            );

            EndPagePrinter(handle)
                .ok()
                .context("EndPagePrinter failed")?;

            EndDocPrinter(handle)
                .ok()
                .context("EndDocPrinter failed")?;
        }

        Ok(())
    })();

    unsafe {
        let _ = ClosePrinter(handle);
    }

    result
}
```
