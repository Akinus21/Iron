//! CAC / Smart Card support via PKCS#11 and p11-kit.
//!
//! WebKitGTK relies on the system's crypto stack (p11-kit + opensc) for
//! client certificate (smart-card) authentication.  On a correctly-configured
//! BlueAK system this is transparent — WebKit will present the smart-card
//! certificate to the remote site automatically.
//!
//! This module offers a status check and a thin setup helper.

use std::path::Path;

const P11_KIT_CONFIG: &str = "/etc/pkcs11/pkcs11.conf";
const OPENSC_MODULE: &str = "/usr/lib/opensc-pkcs11.so";
const OPENSC_MODULE_64: &str = "/usr/lib64/opensc-pkcs11.so";

/// Report whether the system looks configured for CAC / smart-card auth.
pub fn is_system_ready() -> bool {
    Path::new(P11_KIT_CONFIG).exists()
        || Path::new(OPENSC_MODULE).exists()
        || Path::new(OPENSC_MODULE_64).exists()
}

/// Human-readable status message suitable for the command overlay.
pub fn status_text() -> String {
    let mut out = String::from("CAC / Smart Card status:\n\n");
    out.push_str(&format!(
        "  p11-kit config  : {}\n",
        if Path::new(P11_KIT_CONFIG).exists() {
            "found"
        } else {
            "not found"
        }
    ));
    out.push_str(&format!(
        "  opensc module   : {}\n",
        if Path::new(OPENSC_MODULE).exists() || Path::new(OPENSC_MODULE_64).exists() {
            "found"
        } else {
            "not found"
        }
    ));
    if is_system_ready() {
        out.push_str("\nSystem looks ready. WebKit will automatically use the smart card when a site requests client-certificate authentication.\n");
    } else {
        out.push_str("\nSystem does NOT appear configured for smart cards.\n");
        out.push_str("Install p11-kit and opensc, then run :default-browser to register Iron.\n");
    }
    out
}
