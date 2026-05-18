use anyhow::{bail, Context, Result};
use tracing::{debug, info};

#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::Foundation::INVALID_HANDLE_VALUE,
    Win32::Graphics::Printing::{
        ClosePrinter, EnumPrintersW, GetDefaultPrinter, OpenPrinterW, StartDocPrinterW,
        StartPagePrinter, EndPagePrinter, EndDocPrinter, WritePrinter,
        PRINTER_ENUM_LOCAL, PRINTER_ENUM_CONNECTIONS,
        PRINTER_INFO_2W, DOC_INFO_1W, PRINTER_DEFAULTSW,
    },
    Win32::Storage::FileSystem::GENERIC_WRITE,
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
        // Stub for development on Linux
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

    // First call: get buffer size
    unsafe {
        let _ = EnumPrintersW(flags, None, level, None, &mut needed, &mut returned);
    }

    if needed == 0 {
        return Ok(vec![]);
    }

    let mut buf: Vec<u8> = vec![0u8; needed as usize];

    let ok = unsafe {
        EnumPrintersW(
            flags,
            None,
            level,
            Some(&mut buf),
            &mut needed,
            &mut returned,
        )
    };

    if ok.is_err() {
        bail!("EnumPrintersW failed");
    }

    let mut printers = Vec::new();
    let info_size = size_of::<PRINTER_INFO_2W>();

    for i in 0..returned as usize {
        let info_ptr = buf.as_ptr().add(i * info_size) as *const PRINTER_INFO_2W;
        let info = unsafe { &*info_ptr };

        let name = unsafe { info.pPrinterName.to_string().unwrap_or_default() };
        let is_default = default_name.as_deref() == Some(name.as_str());
        let is_online = (info.Status & 0x00000080) == 0; // PRINTER_STATUS_OFFLINE = 0x80

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
    let ok = unsafe { GetDefaultPrinter(Some(&mut buf), &mut size) };
    if ok.is_ok() {
        Some(String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string())
    } else {
        None
    }
}

/// Send raw bytes directly to a printer (ESC/POS, ZPL, or any raw format).
pub fn print_raw(printer_name: &str, data: &[u8]) -> Result<()> {
    info!("Sending {} bytes to printer '{}'", data.len(), printer_name);

    #[cfg(windows)]
    {
        print_raw_windows(printer_name, data)
    }
    #[cfg(not(windows))]
    {
        // Dev stub
        debug!("DEV STUB: Would print {} bytes to '{}'", data.len(), printer_name);
        Ok(())
    }
}

#[cfg(windows)]
fn print_raw_windows(printer_name: &str, data: &[u8]) -> Result<()> {
    use std::ptr;

    // Convert printer name to wide string
    let printer_name_wide: Vec<u16> = printer_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut handle = std::ptr::null_mut();

    let defaults = PRINTER_DEFAULTSW {
        pDatatype: PCWSTR::null(),
        pDevMode: ptr::null_mut(),
        DesiredAccess: GENERIC_WRITE.0,
    };

    unsafe {
        OpenPrinterW(
            PCWSTR(printer_name_wide.as_ptr()),
            &mut handle,
            Some(&defaults),
        )
        .context("OpenPrinterW failed")?;
    }

    if handle == INVALID_HANDLE_VALUE.0 as *mut _ {
        bail!("Failed to open printer '{}'", printer_name);
    }

    // Document name wide string
    let doc_name: Vec<u16> = "PrintBridge Job\0".encode_utf16().collect();
    let datatype: Vec<u16> = "RAW\0".encode_utf16().collect();

    let doc_info = DOC_INFO_1W {
        pDocName: PCWSTR(doc_name.as_ptr()),
        pOutputFile: PCWSTR::null(),
        pDatatype: PCWSTR(datatype.as_ptr()),
    };

    let result = (|| -> Result<()> {
        unsafe {
            StartDocPrinterW(handle, 1, &doc_info as *const _ as *const _)
                .context("StartDocPrinterW failed")?;
            StartPagePrinter(handle).context("StartPagePrinter failed")?;

            let mut written: u32 = 0;
            WritePrinter(
                handle,
                data.as_ptr() as *const _,
                data.len() as u32,
                &mut written,
            )
            .context("WritePrinter failed")?;

            debug!("Wrote {} / {} bytes", written, data.len());

            EndPagePrinter(handle).context("EndPagePrinter failed")?;
            EndDocPrinter(handle).context("EndDocPrinter failed")?;
        }
        Ok(())
    })();

    unsafe { ClosePrinter(handle) };

    result
}
