//! Console port (architecture 4, 8.2): stdout and stderr lines for warnings and output. Trait only; the
//! real adapter arrives with the command wiring (S-17), the fake is `tests/support/fake_console.rs`.

/// FR-OPEN-11, architecture 8.2: each call writes one line; the adapter adds the newline.
pub trait Console {
    fn out(&self, text: &str);
    fn err(&self, text: &str);
}
