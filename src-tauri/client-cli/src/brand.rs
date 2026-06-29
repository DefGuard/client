//! Defguard CLI brand banner -- logo + copyright + version line.
//!
//! Shown when invoked with no arguments or with `--help`.
//! Suppressed for `--version` (which must stay grep-friendly).
//!
//! The logo is emitted on non-Windows platforms only. Its art uses
//! fine-grained Unicode block glyphs (eighths/quadrant blocks) that many
//! Windows console fonts can't render, so Windows shows just the
//! copyright + version line.
//!
//! Two assets (non-Windows):
//!   - assets/logo-color.ansi -- ANSI block-character art,
//!     used when stdout is an interactive TTY
//!   - assets/logo-mono.txt   -- plain ASCII fallback (no ANSI),
//!     used when output is piped/redirected or NO_COLOR is set
//!
//! Both assets are embedded at compile time via `include_str!`.

#[cfg(not(windows))]
use owo_colors::{OwoColorize, Stream};

#[cfg(not(windows))]
const LOGO_COLOR: &str = include_str!("../assets/logo-color.ansi");
#[cfg(not(windows))]
const LOGO_MONO: &str = include_str!("../assets/logo-mono.txt");

const COPYRIGHT: &str = "Copyright (C) 2026 Defguard Sp. z o.o.";

/// Print logo + copyright + project name/version to stdout. Picks the
/// colored logo variant on an interactive TTY and the mono fallback
/// otherwise (so `defguard-cli --help | cat` stays clean ASCII).
#[cfg(not(windows))]
pub fn print_banner() {
    // owo-colors' supports-colors detection drives the choice: if
    // stdout supports color, emit the ANSI variant; otherwise mono.
    // We do not feed the logo through if_supports_color directly --
    // it carries its own ANSI escapes -- we just gate which string
    // we emit. NO_COLOR / CLICOLOR_FORCE propagate via the
    // supports_color() helper.
    let use_color = "x"
        .if_supports_color(Stream::Stdout, |s| s.red())
        .to_string()
        != "x";

    let logo = if use_color { LOGO_COLOR } else { LOGO_MONO };
    println!("{logo}");

    let project = common::version_string("defguard-cli");
    if use_color {
        println!("    {}", project.bright_yellow().bold());
        println!("    {}", COPYRIGHT.dimmed());
    } else {
        println!("    {project}");
        println!("    {COPYRIGHT}");
    }
    println!();
}

/// Print copyright + project name/version to stdout. The logo is skipped
/// on Windows: its art relies on Unicode block glyphs that many Windows
/// console fonts can't render, and the console may not interpret ANSI.
#[cfg(windows)]
pub fn print_banner() {
    let project = common::version_string("defguard-cli");
    println!();
    println!("    {project}");
    println!("    {COPYRIGHT}");
    println!();
}
