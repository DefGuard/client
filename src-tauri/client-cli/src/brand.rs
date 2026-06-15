//! Defguard CLI brand banner -- logo + copyright + version line.
//!
//! Shown when invoked with no arguments or with `--help`.
//! Suppressed for `--version` (which must stay grep-friendly).
//!
//! Two assets:
//!   - assets/logo-color.ansi -- ANSI block-character art,
//!     used when stdout is an interactive TTY
//!   - assets/logo-mono.txt   -- plain ASCII fallback (no ANSI),
//!     used when output is piped/redirected or NO_COLOR is set
//!
//! Both assets are embedded at compile time via `include_str!`.

use owo_colors::{OwoColorize, Stream};

const LOGO_COLOR: &str = include_str!("../assets/logo-color.ansi");
const LOGO_MONO: &str = include_str!("../assets/logo-mono.txt");

const COPYRIGHT: &str = "Copyright (C) 2026 Defguard Sp. z o.o.";

/// Print logo + copyright + project name/version to stdout. Picks
/// the colored variant on an interactive TTY, the mono fallback
/// otherwise (so `defguard-cli --help | cat` stays clean ASCII).
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
        let p = project.bright_yellow().bold().to_string();
        let c = COPYRIGHT.dimmed().to_string();
        println!("    {p}");
        println!("    {c}");
    } else {
        println!("    {project}");
        println!("    {COPYRIGHT}");
    }
    println!();
}
