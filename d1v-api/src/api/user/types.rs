use crate::validate::{UrlError, Validate};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::fmt;
use std::fmt::{Display, Formatter};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    #[serde(default)]
    pub slug: String,
    pub is_agent: bool,
    pub picture: String,
    #[serde(default)]
    pub is_onboarded: bool,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub is_super_admin: bool,
    #[serde(default)]
    pub is_company: bool,
    #[serde(default)]
    pub company_name: String,
    #[serde(default)]
    pub company_website: String,
    #[serde(default)]
    pub industry: String,
    #[serde(default)]
    pub invite_code: String,
    #[serde(default)]
    pub referral_code: String,
    #[serde(default)]
    pub sol_wallet: String,
    #[serde(default)]
    pub sui_wallet: String,
    #[serde(default)]
    pub evm_wallet: String,
    #[serde(default)]
    pub sub: String,
    pub email: Option<String>,
    pub last_login_type: Option<String>,
    pub stripe_customer_id: Option<String>,
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)?;

        if !self.slug.is_empty() {
            write!(f, " ({})", self.slug)?;
        }

        if let Some(email) = &self.email
            && !email.is_empty()
        {
            write!(f, " <{email}>")?;
        }

        Ok(())
    }
}

// Note: do not add `is_company` here. The server ignores it.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateUser {
    pub company_name: Option<String>,
    pub company_website: Option<String>,
    /// `None` leaves the picture unchanged; `Some("")` clears it on the server.
    pub picture: Option<String>,
    pub industry: Option<String>,
    pub referral_code: Option<String>,
}

impl Validate for UpdateUser {
    type Error = UrlError;

    fn validate(&self) -> Result<(), Self::Error> {
        if let Some(url) = &self.company_website {
            Url::parse(url).map_err(|_| UrlError::Invalid)?;
        }
        if let Some(url) = &self.picture {
            Url::parse(url).map_err(|_| UrlError::Invalid)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDailyActivity {
    pub start_date: String,
    pub end_date: String,
    pub days: i32,
    pub counts: Vec<DailyCount>,
}

impl Display for PromptDailyActivity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ~ {} ({} days)",
            self.start_date, self.end_date, self.days
        )?;

        for entry in &self.counts {
            write!(f, "\n  {}: {}", entry.date, entry.count)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCount {
    pub date: String,
    pub count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_user_valid_urls() {
        let update = UpdateUser {
            company_website: Some("https://example.com".into()),
            picture: Some("https://cdn.example.com/pic.jpg".into()),
            ..Default::default()
        };
        assert!(update.validate().is_ok());
    }

    #[test]
    fn update_user_none_urls() {
        let update = UpdateUser::default();
        assert!(update.validate().is_ok());
    }

    #[test]
    fn update_user_invalid_website() {
        let update = UpdateUser {
            company_website: Some("not-a-url".into()),
            ..Default::default()
        };
        assert!(update.validate().is_err());
    }

    #[test]
    fn update_user_invalid_picture() {
        let update = UpdateUser {
            picture: Some("not-a-url".into()),
            ..Default::default()
        };
        assert!(update.validate().is_err());
    }
}
