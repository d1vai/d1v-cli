use const_format::concatc;

pub const SUCCESS: &str = "✓";
pub const ERROR: &str = "✗";
pub const INFO: &str = "→";
pub const PROMPT: &str = "?";
pub const SELECT: &str = "◆";
pub const SELECT_ARROW: &str = "❯";

pub const SUCCESS_PREFIX: &str = concatc!(SUCCESS, " ");
pub const ERROR_PREFIX: &str = concatc!(ERROR, " ");
pub const INFO_PREFIX: &str = concatc!(INFO, " ");
pub const PROMPT_PREFIX: &str = concatc!(PROMPT, " ");
pub const SELECT_PREFIX: &str = concatc!(SELECT, " ");
