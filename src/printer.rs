use anyhow::{Context, Result};
use tracing::{debug, info};

#[cfg(windows)]
use anyhow::bail;
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::HANDLE,
        Graphics::Printing::{
            ClosePrinter, DOC_INFO_1W, EndDocPrinter, EndPagePrinter,
            EnumPrintersW, GetDefaultPrinterW, OpenPrinterW,
            PRINTER_ACCESS_USE, PRINTER_DEFAULTSW,
            PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
            PRINTER_INFO_2W, StartDocPrinterW, StartPagePrinter,
            WritePrinter,
        },
    },
};

#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
    pub is_online: bool,
}

/// List all printers
pub fn list_printers() -> Result<Vec<PrinterInfo>> {
    #[cfg(windows)]
    {
        list_printers_windows()
    }

    #[cfg(not(windows))]
    {
        // Simple Linux support for USB thermal printers
        let mut printers = vec![];
        if std::path::Path::new("/dev/usb/lp0").exists() {
            printers.push(PrinterInfo {
                name: "/dev/usb/lp0".to_string(),
                is_default: true,
                is_online: true,
            });
        }
        
        if printers.is_empty() {
            printers.push(PrinterInfo {
                name: "DEV_PRINTER".to_string(),
                is_default: true,
                is_online: true,
            });
        }
        
        Ok(printers)
    }
}

#[cfg(windows)]
fn list_printers_windows() -> Result<Vec<PrinterInfo>> {
    let default_name = get_default_printer_name();

    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    let level: u32 = 2;

    let mut needed: u32 = 0;
    let mut returned: u32 = 0;

    // First call to get buffer size
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

    let mut buf = vec![0u8; needed as usize];

    unsafe {
        EnumPrintersW(
            flags,
            PCWSTR::null(),
            level,
            Some(buf.as_mut_slice()),
            &mut needed,
            &mut returned,
        )
        .ok()
        .context("EnumPrintersW failed")?;
    }

    let mut printers = Vec::new();

    let printer_info_ptr = buf.as_ptr() as *const PRINTER_INFO_2W;

    for i in 0..returned as usize {
        let info = unsafe { &*printer_info_ptr.add(i) };

        let name = unsafe {
            info.pPrinterName
                .to_string()
                .unwrap_or_else(|_| "Unknown".to_string())
        };

        // PRINTER_STATUS_OFFLINE = 0x00000080
        let is_online = (info.Status & 0x00000080) == 0;

        let is_default =
            default_name.as_deref() == Some(name.as_str());

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
    let mut needed: u32 = 256;

    let mut buf = vec![0u16; needed as usize];

    let success = unsafe {
        GetDefaultPrinterW(PWSTR(buf.as_mut_ptr()), &mut needed)
    };

    if success.as_bool() {
        Some(
            String::from_utf16_lossy(&buf)
                .trim_end_matches('\0')
                .to_string(),
        )
    } else {
        None
    }
}

/// Print raw ESC/POS bytes
pub fn print_raw(
    printer_name: &str,
    data: &[u8],
) -> Result<()> {
    info!(
        "Printing {} bytes to '{}'",
        data.len(),
        printer_name
    );

    #[cfg(windows)]
    {
        print_raw_windows(printer_name, data)
    }

    #[cfg(not(windows))]
    {
        if printer_name == "/dev/usb/lp0" || printer_name == "default" {
            // Direct write to Linux USB character device
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/usb/lp0")
                .context("Failed to open /dev/usb/lp0 - Are you in the 'lp' group or root?")?;
            
            file.write_all(data).context("Failed to write to printer device")?;
            file.flush().context("Failed to flush printer device")?;
            debug!("Successfully printed to Linux USB printer");
        } else {
            debug!("DEV MODE print: ignored for {}", printer_name);
        }

        Ok(())
    }
}

#[cfg(windows)]
fn print_raw_windows(
    printer_name: &str,
    data: &[u8],
) -> Result<()> {
    let printer_name_wide: Vec<u16> = printer_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut handle = HANDLE::default();

    let defaults = PRINTER_DEFAULTSW {
        pDatatype: PWSTR::null(),
        pDevMode: std::ptr::null_mut(),
        DesiredAccess: PRINTER_ACCESS_USE,
    };

    unsafe {
        OpenPrinterW(
            PCWSTR(printer_name_wide.as_ptr()),
            &mut handle,
            Some(&defaults),
        )
        .ok()
        .context("OpenPrinterW failed")?;
    }

    if handle.is_invalid() {
        bail!("Printer handle invalid");
    }

    let mut doc_name: Vec<u16> =
        "PrintBridge Job\0".encode_utf16().collect();

    let mut raw: Vec<u16> =
        "RAW\0".encode_utf16().collect();

    let doc_info = DOC_INFO_1W {
        pDocName: PWSTR(doc_name.as_mut_ptr()),
        pOutputFile: PWSTR::null(),
        pDatatype: PWSTR(raw.as_mut_ptr()),
    };

    let result: Result<()> = (|| {
        unsafe {
            let job = StartDocPrinterW(
                handle,
                1,
                &doc_info as *const _ as *const _,
            );

            if job == 0 {
                bail!("StartDocPrinterW failed");
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

            debug!("Written {} bytes", written);

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

