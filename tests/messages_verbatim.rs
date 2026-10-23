//! Literal user-facing strings from requirements.md "Messages" that exist at S-00.
//! Each later slice adds its own literals here (docs/testing.md, level 5). A message change breaks
//! this file on purpose.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::messages;

// FR-CMD-08, message "Cancelled." (stdout, exit 0)
#[test]
fn cancelled_text_is_verbatim() {
    assert_eq!(messages::cancelled(), "Cancelled.");
}
