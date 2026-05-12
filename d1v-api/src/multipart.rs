use std::borrow::Cow;
use std::fmt::Display;

use reqwest::multipart::Form;

pub trait FormExt: Sized {
    fn text_if<T, U>(self, name: T, value: Option<U>) -> Self
    where
        T: Into<Cow<'static, str>>,
        U: Into<Cow<'static, str>>;

    fn text_if_display<T, U>(self, name: T, value: Option<U>) -> Self
    where
        T: Into<Cow<'static, str>>,
        U: Display;
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

    fn text_if_display<T, U>(self, name: T, value: Option<U>) -> Self
    where
        T: Into<Cow<'static, str>>,
        U: Display,
    {
        if let Some(value) = value {
            self.text(name, value.to_string())
        } else {
            self
        }
    }
}
