use std::fmt;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

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
