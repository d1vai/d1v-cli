use crate::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T> {
    pub code: i64,
    #[serde(rename = "msg")]
    pub message: String,
    pub data: Option<T>,
    pub total: Option<i64>,
}

impl<T> Response<T> {
    pub fn ok(self) -> Result<T, Error> {
        self.into()
    }
}

impl<T> From<Response<T>> for Result<T, Error> {
    fn from(
        Response {
            code,
            message,
            data,
            ..
        }: Response<T>,
    ) -> Self {
        if code != 0 {
            return Err(Error::Api { code, message });
        }

        data.ok_or(Error::MissingData)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Location {
    String(String),
    Integer(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetail {
    #[serde(rename = "loc")]
    pub location: Vec<Location>,
    #[serde(rename = "msg")]
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub struct ValidationError {
    pub detail: Vec<ValidationDetail>,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "validation errors:")?;

        for ValidationDetail {
            location,
            message,
            error_type,
        } in &self.detail
        {
            let location = location
                .iter()
                .map(|l| match l {
                    Location::String(s) => s.to_string(),
                    Location::Integer(i) => i.to_string(),
                })
                .collect::<Vec<_>>()
                .join(".");

            writeln!(f)?;
            write!(f, "{message} [type={error_type}, location={location}]")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let json = r#"{"code": 0, "msg": "ok", "data": "success", "total": null}"#;
        let resp: Response<String> = serde_json::from_str(json).unwrap();

        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "ok");
        assert_eq!(resp.total, None);
        assert_eq!(resp.ok().unwrap(), "success");
    }

    #[test]
    fn test_api_error() {
        let json = r#"{"code": 401, "msg": "unauthorized", "data": null, "total": null}"#;
        let resp: Response<()> = serde_json::from_str(json).unwrap();

        assert_eq!(
            resp.ok().unwrap_err().to_string(),
            "api error 401: unauthorized"
        );
    }

    #[test]
    fn test_missing_data() {
        let json = r#"{"code": 0, "msg": "ok"}"#;
        let resp: Response<String> = serde_json::from_str(json).unwrap();

        assert_eq!(resp.ok().unwrap_err().to_string(), "missing data");
    }

    #[test]
    fn test_validation_error() {
        let json = r#"{
            "detail": [
                {
                    "loc": ["query", "email"],
                    "msg": "Field required",
                    "type": "missing"
                },
                {
                    "loc": ["body", "verify_code"],
                    "msg": "Field required",
                    "type": "missing"
                }
            ]
        }"#;
        let err: ValidationError = serde_json::from_str(json).unwrap();

        assert_eq!(
            err.to_string(),
            concat!(
                "validation errors:\n",
                "Field required [type=missing, location=query.email]\n",
                "Field required [type=missing, location=body.verify_code]"
            )
        );
    }
}
