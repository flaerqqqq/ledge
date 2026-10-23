//! User-facing strings (architecture 8.3). One const or fn per text, verified literally in
//! tests/messages_verbatim.rs.

/// FR-CMD-08, message "Cancelled." (stdout, exit 0).
pub fn cancelled() -> &'static str {
    "Cancelled."
}

/// FR-PLAT-02, architecture 9.1: no usable XDG value and no `HOME`.
pub fn home_not_set() -> &'static str {
    "HOME is not set."
}
