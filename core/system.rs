use crate::error::Result; // Removed AppError from here
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use std::env;
use sysinfo::System;

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "camelCase"))]
pub struct SystemInfo {
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    os_name: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    os_version: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    kernel_version: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    hostname: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    shell: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    term: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    error: Option<String>, // Keep for potential errors during collection
}

pub fn gather_system_info() -> Result<SystemInfo> {
    // Keep Result for consistency
    let mut info = SystemInfo::default();
    let mut sys = System::new_all();

    // Removed catch_unwind
    sys.refresh_all(); // Call directly

    info.shell = env::var("SHELL").ok();
    info.term = env::var("TERM").ok();

    info.os_name = System::name();
    info.os_version = System::os_version();
    info.kernel_version = System::kernel_version();
    info.hostname = System::host_name();

    // Check if essential info is missing, potentially indicating an issue
    if info.os_name.is_none() && info.hostname.is_none() {
        info.error = Some("Failed to retrieve OS name and hostname.".to_string());
        // Return info with error message
        return Ok(info);
    }

    Ok(info)
}
