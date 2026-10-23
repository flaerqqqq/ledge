//! Environment variables and the working directory (architecture 4). Trait only; the real adapter
//! is `adapters::env::RealEnv` (S-01), the fake is `tests/support/fake_env.rs`.

/// FR-PLAT-02, FR-CONF-08: read-only view of the process environment.
pub trait Env {
    /// Value of a variable. An empty value is returned as set; callers decide what it means.
    fn var(&self, name: &str) -> Option<String>;
    /// Current working directory as text (made absolute and cleaned by the caller).
    fn cwd(&self) -> std::io::Result<String>;
}
