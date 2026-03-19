use crate::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub code: i64,
    #[serde(rename = "msg")]
    pub message: String,
    #[serde(default)]
    pub data: Value,
    pub total: Option<i64>,
}

impl Response {
    pub fn ok<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        if self.code != 0 {
            return Err(Error::Api {
                code: self.code,
                message: self.message,
            });
        }

        serde_json::from_value(self.data).map_err(Error::Data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let json = r#"{"code": 0, "msg": "success", "data": "success", "total": null}"#;
        let resp: Response = serde_json::from_str(json).unwrap();

        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "success");
        assert_eq!(resp.total, None);
        assert_eq!(resp.ok::<String>().unwrap(), "success");

        let json = r#"{"code": 0, "msg": "success", "data": null, "total": null}"#;
        let resp: Response = serde_json::from_str(json).unwrap();

        assert!(resp.ok::<()>().is_ok());
    }

    #[test]
    fn api_error() {
        let json = r#"{"code": 401, "msg": "unauthorized", "data": null, "total": null}"#;
        let resp: Response = serde_json::from_str(json).unwrap();

        assert_eq!(
            resp.clone().ok::<String>().unwrap_err().to_string(),
            "api error 401: unauthorized"
        );
        assert_eq!(
            resp.ok::<()>().unwrap_err().to_string(),
            "api error 401: unauthorized"
        );
    }

    #[test]
    fn invalid_data() {
        let json = r#"{"code": 0, "msg": "ok"}"#;
        let resp: Response = serde_json::from_str(json).unwrap();

        assert_eq!(
            resp.ok::<String>().unwrap_err().to_string(),
            "invalid response data: invalid type: null, expected a string"
        );
    }
}
