use crate::Error;
use serde::{Deserialize, Serialize};

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
}
