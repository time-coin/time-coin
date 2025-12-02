//! Response utilities and macros for consistent API responses

/// Macro to create JSON responses consistently
///
/// # Example
/// ```
/// use crate::response::ok_json;
///
/// ok_json!(BlockchainInfo {
///     chain: "TIME".to_string(),
///     blocks: 100,
/// })
/// ```
#[macro_export]
macro_rules! ok_json {
    ($data:expr) => {
        Ok(axum::Json($data))
    };
}

/// Helper trait for converting results to API results
#[allow(dead_code)]
pub trait IntoApiResult<T> {
    fn into_api_result(self) -> crate::ApiResult<axum::Json<T>>;
}

impl<T, E> IntoApiResult<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn into_api_result(self) -> crate::ApiResult<axum::Json<T>> {
        match self {
            Ok(data) => Ok(axum::Json(data)),
            Err(e) => Err(crate::ApiError::Internal(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_json_macro() {
        // Macro should compile and create Ok(Json(data))
        #[derive(serde::Serialize)]
        struct TestData {
            value: i32,
        }

        let result: crate::ApiResult<axum::Json<TestData>> = ok_json!(TestData { value: 42 });
        assert!(result.is_ok());
    }
}
