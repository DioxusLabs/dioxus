use super::text_like::{TextLike, TextLikeController};

pub(crate) type Password = TextLike<PasswordController>;

#[derive(Debug, Default)]
pub(crate) struct PasswordController;

impl TextLikeController for PasswordController {
    fn display_text(&self, text: &str) -> String {
        text.chars().map(|_| '.').collect()
    }
}
