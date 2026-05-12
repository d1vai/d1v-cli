use std::borrow::Cow;

use reqwest::multipart::Form;

pub trait FormExt: Sized {
    fn text_if<T, U>(self, name: T, value: Option<U>) -> Self
    where
        T: Into<Cow<'static, str>>,
        U: Into<Cow<'static, str>>;
}

impl FormExt for Form {
    fn text_if<T, U>(self, name: T, value: Option<U>) -> Self
    where
        T: Into<Cow<'static, str>>,
        U: Into<Cow<'static, str>>,
    {
        if let Some(value) = value {
            self.text(name, value)
        } else {
            self
        }
    }
}
