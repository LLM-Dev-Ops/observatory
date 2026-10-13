///! Advanced filtering models for trace search
///!
///! This module provides enterprise-grade filtering with:
///! - Comparison operators (gt, gte, lt, lte, eq, ne)
///! - Collection operators (in, not_in, contains, not_contains)
///! - Logical operators (AND, OR, NOT)
///! - Full-text search support
///! - SQL injection prevention
///! - Filter validation
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Filter operator for comparisons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FilterOperator {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal to
    Gte,
    /// Less than
    Lt,
    /// Less than or equal to
    Lte,
    /// In a list of values
    In,
    /// Not in a list of values
    NotIn,
    /// Contains substring (case-insensitive)
    Contains,
    /// Does not contain substring (case-insensitive)
    NotContains,
    /// Starts with prefix
    StartsWith,
    /// Ends with suffix
    EndsWith,
    /// Matches regex pattern
    Regex,
    /// Full-text search
    Search,
}

impl fmt::Display for FilterOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterOperator::Eq => write!(f, "="),
            FilterOperator::Ne => write!(f, "!="),
            FilterOperator::Gt => write!(f, ">"),
            FilterOperator::Gte => write!(f, ">="),
            FilterOperator::Lt => write!(f, "<"),
            FilterOperator::Lte => write!(f, "<="),
            FilterOperator::In => write!(f, "IN"),
            FilterOperator::NotIn => write!(f, "NOT IN"),
            FilterOperator::Contains => write!(f, "ILIKE"),
            FilterOperator::NotContains => write!(f, "NOT ILIKE"),
            FilterOperator::StartsWith => write!(f, "ILIKE"),
            FilterOperator::EndsWith => write!(f, "ILIKE"),
            FilterOperator::Regex => write!(f, "~*"),
            FilterOperator::Search => write!(f, "@@"),
        }
    }
}

/// Filter value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    DateTime(DateTime<Utc>),
    Array(Vec<String>),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
    Null,
}

impl FilterValue {
    /// Convert to SQL parameter value
    pub fn to_sql_string(&self) -> String {
        match self {
            FilterValue::String(s) => format!("'{}'", s.replace('\'', "''")),
            FilterValue::Int(i) => i.to_string(),
            FilterValue::Float(f) => f.to_string(),
            FilterValue::Bool(b) => b.to_string(),
            FilterValue::DateTime(dt) => format!("'{}'", dt.to_rfc3339()),
            FilterValue::Array(arr) => {
                let values: Vec<String> = arr
                    .iter()
                    .map(|s| format!("'{}'", s.replace('\'', "''")))
                    .collect();
                format!("({})", values.join(", "))
            }
            FilterValue::IntArray(arr) => {
                let values: Vec<String> = arr.iter().map(|i| i.to_string()).collect();
                format!("({})", values.join(", "))
            }
            FilterValue::FloatArray(arr) => {
                let values: Vec<String> = arr.iter().map(|f| f.to_string()).collect();
                format!("({})", values.join(", "))
            }
            FilterValue::Null => "NULL".to_string(),
        }
    }

    /// Check if value is valid for the operator
    pub fn is_valid_for_operator(&self, operator: &FilterOperator) -> bool {
        match operator {
            FilterOperator::In | FilterOperator::NotIn => {
                matches!(
                    self,
                    FilterValue::Array(_) | FilterValue::IntArray(_) | FilterValue::FloatArray(_)
                )
            }
            FilterOperator::Contains
            | FilterOperator::NotContains
            | FilterOperator::StartsWith
            | FilterOperator::EndsWith
            | FilterOperator::Regex
            | FilterOperator::Search => {
                matches!(self, FilterValue::String(_))
            }
            FilterOperator::Gt | FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
                matches!(
                    self,
                    FilterValue::Int(_) | FilterValue::Float(_) | FilterValue::DateTime(_)
                )
            }
            FilterOperator::Eq | FilterOperator::Ne => true,
        }
    }
}

/// Field filter with operator and value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldFilter {
    /// Field name to filter on
    pub field: String,
    /// Filter operator
    pub operator: FilterOperator,
    /// Filter value
    pub value: FilterValue,
}

impl FieldFilter {
    /// Validate the filter
    pub fn validate(&self) -> Result<(), String> {
        // Validate field name (prevent SQL injection)
        if !self.is_valid_field_name(&self.field) {
            return Err(format!("Invalid field name: {}", self.field));
        }

        // Validate value for operator
        if !self.value.is_valid_for_operator(&self.operator) {
            return Err(format!(
                "Invalid value type for operator {:?}",
                self.operator
            ));
        }

        Ok(())
    }

    /// Check if field name is valid (whitelist approach)
    fn is_valid_field_name(&self, field: &str) -> bool {
        matches!(
            field,
            "ts" | "trace_id"
                | "span_id"
                | "parent_span_id"
                | "project_id"
                | "session_id"
                | "user_id"
                | "provider"
                | "model"
                | "operation_type"
                | "input_text"
                | "output_text"
                | "prompt_tokens"
                | "completion_tokens"
                | "total_tokens"
                | "input_cost_usd"
                | "output_cost_usd"
                | "total_cost_usd"
                | "duration_ms"
                | "latency_ms"
                | "time_to_first_token_ms"
                | "tokens_per_second"
                | "status_code"
                | "error_message"
                | "environment"
                | "tags"
                | "metadata"
                | "model_parameters"
        )
    }

    /// Convert to SQL WHERE clause
    pub fn to_sql(&self, param_index: &mut i32) -> Result<(String, Vec<String>), String> {
        self.validate()?;

        let field = &self.field;
        let mut params = Vec::new();

        let condition = match &self.operator {
            FilterOperator::Eq => {
                params.push(self.value.to_sql_string());
                format!("{} = ${}", field, param_index)
            }
            FilterOperator::Ne => {
                params.push(self.value.to_sql_string());
                format!("{} != ${}", field, param_index)
            }
            FilterOperator::Gt => {
                params.push(self.value.to_sql_string());
                format!("{} > ${}", field, param_index)
            }
            FilterOperator::Gte => {
                params.push(self.value.to_sql_string());
                format!("{} >= ${}", field, param_index)
            }
            FilterOperator::Lt => {
                params.push(self.value.to_sql_string());
                format!("{} < ${}", field, param_index)
            }
            FilterOperator::Lte => {
                params.push(self.value.to_sql_string());
                format!("{} <= ${}", field, param_index)
            }
            FilterOperator::In => {
                if let FilterValue::Array(arr) = &self.value {
                    let placeholders: Vec<String> = (0..arr.len())
                        .map(|i| {
                            let idx = *param_index + i as i32;
                            format!("${}", idx)
                        })
                        .collect();
                    params.extend(arr.iter().map(|s| format!("'{}'", s.replace('\'', "''"))));
                    *param_index += arr.len() as i32 - 1;
                    format!("{} IN ({})", field, placeholders.join(", "))
                } else {
                    return Err("IN operator requires array value".to_string());
                }
            }
            FilterOperator::NotIn => {
                if let FilterValue::Array(arr) = &self.value {
                    let placeholders: Vec<String> = (0..arr.len())
                        .map(|i| {
                            let idx = *param_index + i as i32;
                            format!("${}", idx)
                        })
                        .collect();
                    params.extend(arr.iter().map(|s| format!("'{}'", s.replace('\'', "''"))));
                    *param_index += arr.len() as i32 - 1;
                    format!("{} NOT IN ({})", field, placeholders.join(", "))
                } else {
                    return Err("NOT IN operator requires array value".to_string());
                }
            }
            FilterOperator::Contains => {
                if let FilterValue::String(s) = &self.value {
                    params.push(format!("%{}%", s));
                    format!("{} ILIKE ${}", field, param_index)
                } else {
                    return Err("CONTAINS operator requires string value".to_string());
                }
            }
            FilterOperator::NotContains => {
                if let FilterValue::String(s) = &self.value {
                    params.push(format!("%{}%", s));
                    format!("{} NOT ILIKE ${}", field, param_index)
                } else {
                    return Err("NOT CONTAINS operator requires string value".to_string());
                }
            }
            FilterOperator::StartsWith => {
                if let FilterValue::String(s) = &self.value {
                    params.push(format!("{}%", s));
                    format!("{} ILIKE ${}", field, param_index)
                } else {
                    return Err("STARTS WITH operator requires string value".to_string());
                }
            }
            FilterOperator::EndsWith => {
                if let FilterValue::String(s) = &self.value {
                    params.push(format!("%{}", s));
                    format!("{} ILIKE ${}", field, param_index)
                } else {
                    return Err("ENDS WITH operator requires string value".to_string());
                }
            }
            FilterOperator::Regex => {
                if let FilterValue::String(s) = &self.value {
                    params.push(s.clone());
                    format!("{} ~* ${}", field, param_index)
                } else {
                    return Err("REGEX operator requires string value".to_string());
                }
            }
            FilterOperator::Search => {
                if let FilterValue::String(s) = &self.value {
                    params.push(s.clone());

                    // Use pre-computed tsvector columns with GIN indexes for optimal performance
                    // Maps field names to their corresponding tsvector columns
                    let search_column = match field.as_str() {
                        "input_text" => "input_text_search",
                        "output_text" => "output_text_search",
                        // For any other field or when searching across both, use combined content_search
                        _ => "content_search",
                    };

                    // Use plainto_tsquery for simple query parsing (handles multi-word queries)
                    // The GIN index on the tsvector column makes this very fast
                    format!(
                        "{} @@ plainto_tsquery('english', ${})",
                        search_column, param_index
                    )
                } else {
                    return Err("SEARCH operator requires string value".to_string());
                }
            }
        };

        *param_index += 1;
        Ok((condition, params))
    }
}

/// Logical operator for combining filters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

/// Complex filter with logical operators
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    /// Single field filter
    Field(FieldFilter),
    /// Logical combination of filters
    Logical {
        operator: LogicalOperator,
        filters: Vec<Filter>,
    },
}

impl Filter {
    /// Validate the filter recursively
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Filter::Field(field_filter) => field_filter.validate(),
            Filter::Logical { operator, filters } => {
                if filters.is_empty() {
                    return Err("Logical operator requires at least one filter".to_string());
                }

                if *operator == LogicalOperator::Not && filters.len() != 1 {
                    return Err("NOT operator requires exactly one filter".to_string());
                }

                for filter in filters {
                    filter.validate()?;
                }

                Ok(())
            }
        }
    }

    /// Convert to SQL WHERE clause
    pub fn to_sql(&self, param_index: &mut i32) -> Result<(String, Vec<String>), String> {
        self.validate()?;

        match self {
            Filter::Field(field_filter) => field_filter.to_sql(param_index),
            Filter::Logical { operator, filters } => {
                let mut conditions = Vec::new();
                let mut all_params = Vec::new();

                for filter in filters {
                    let (condition, params) = filter.to_sql(param_index)?;
                    conditions.push(format!("({})", condition));
                    all_params.extend(params);
                }

                let combined = match operator {
                    LogicalOperator::And => conditions.join(" AND "),
                    LogicalOperator::Or => conditions.join(" OR "),
                    LogicalOperator::Not => format!("NOT ({})", conditions[0]),
                };

                Ok((combined, all_params))
            }
        }
    }
}

/// Advanced search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchRequest {
    /// Complex filter expression
    pub filter: Option<Filter>,

    /// Sort field
    pub sort_by: Option<String>,

    /// Sort direction
    #[serde(default)]
    pub sort_desc: bool,

    /// Pagination cursor
    pub cursor: Option<String>,

    /// Result limit (1-1000)
    #[serde(default = "default_limit")]
    pub limit: i32,

    /// Fields to include in response
    pub fields: Option<Vec<String>>,
}

fn default_limit() -> i32 {
    50
}

impl AdvancedSearchRequest {
    /// Validate the search request
    pub fn validate(&self) -> Result<(), String> {
        // Validate limit
        if self.limit < 1 || self.limit > 1000 {
            return Err(format!(
                "Limit must be between 1 and 1000, got {}",
                self.limit
            ));
        }

        // Validate filter if present
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }

        // Validate sort field
        if let Some(sort_by) = &self.sort_by {
            if !self.is_valid_sort_field(sort_by) {
                return Err(format!("Invalid sort field: {}", sort_by));
            }
        }

        // Validate field selection
        if let Some(fields) = &self.fields {
            for field in fields {
                if !self.is_valid_field_name(field) {
                    return Err(format!("Invalid field name: {}", field));
                }
            }
        }

        Ok(())
    }

    fn is_valid_sort_field(&self, field: &str) -> bool {
        matches!(
            field,
            "ts" | "trace_id" | "duration_ms" | "total_cost_usd" | "total_tokens"
        )
    }

    fn is_valid_field_name(&self, field: &str) -> bool {
        matches!(
            field,
            "ts" | "trace_id"
                | "span_id"
                | "provider"
                | "model"
                | "input_text"
                | "output_text"
                | "total_tokens"
                | "total_cost_usd"
                | "duration_ms"
                | "status_code"
                | "environment"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_filter_validation() {
        let filter = FieldFilter {
            field: "provider".to_string(),
            operator: FilterOperator::Eq,
            value: FilterValue::String("openai".to_string()),
        };
        assert!(filter.validate().is_ok());

        // Invalid field name
        let invalid_filter = FieldFilter {
            field: "invalid_field; DROP TABLE traces;".to_string(),
            operator: FilterOperator::Eq,
            value: FilterValue::String("test".to_string()),
        };
        assert!(invalid_filter.validate().is_err());
    }

    #[test]
    fn test_filter_operator_sql() {
        let filter = FieldFilter {
            field: "total_cost_usd".to_string(),
            operator: FilterOperator::Gte,
            value: FilterValue::Float(1.0),
        };

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, _params) = result.unwrap();
        assert!(sql.contains(">="));
    }

    #[test]
    fn test_logical_filter() {
        let filter = Filter::Logical {
            operator: LogicalOperator::And,
            filters: vec![
                Filter::Field(FieldFilter {
                    field: "provider".to_string(),
                    operator: FilterOperator::Eq,
                    value: FilterValue::String("openai".to_string()),
                }),
                Filter::Field(FieldFilter {
                    field: "total_cost_usd".to_string(),
                    operator: FilterOperator::Gt,
                    value: FilterValue::Float(1.0),
                }),
            ],
        };

        assert!(filter.validate().is_ok());

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, _params) = result.unwrap();
        assert!(sql.contains("AND"));
    }

    #[test]
    fn test_in_operator() {
        let filter = FieldFilter {
            field: "provider".to_string(),
            operator: FilterOperator::In,
            value: FilterValue::Array(vec!["openai".to_string(), "anthropic".to_string()]),
        };

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("IN"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_contains_operator() {
        let filter = FieldFilter {
            field: "input_text".to_string(),
            operator: FilterOperator::Contains,
            value: FilterValue::String("hello".to_string()),
        };

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("ILIKE"));
        assert_eq!(params.len(), 1);
        assert!(params[0].contains("%hello%"));
    }

    #[test]
    fn test_search_operator_input_text() {
        // Test full-text search on input_text field
        let filter = FieldFilter {
            field: "input_text".to_string(),
            operator: FilterOperator::Search,
            value: FilterValue::String("authentication error".to_string()),
        };

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();

        // Should use input_text_search tsvector column
        assert!(sql.contains("input_text_search"));
        assert!(sql.contains("@@"));
        assert!(sql.contains("plainto_tsquery"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "authentication error");
    }

    #[test]
    fn test_search_operator_output_text() {
        // Test full-text search on output_text field
        let filter = FieldFilter {
            field: "output_text".to_string(),
            operator: FilterOperator::Search,
            value: FilterValue::String("success response".to_string()),
        };

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();

        // Should use output_text_search tsvector column
        assert!(sql.contains("output_text_search"));
        assert!(sql.contains("@@"));
        assert!(sql.contains("plainto_tsquery"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_search_operator_combined() {
        // Test full-text search across both input and output (default)
        let filter = FieldFilter {
            field: "content".to_string(), // Non-standard field triggers combined search
            operator: FilterOperator::Search,
            value: FilterValue::String("error message".to_string()),
        };

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();

        // Should use content_search tsvector column (combined)
        assert!(sql.contains("content_search"));
        assert!(sql.contains("@@"));
        assert!(sql.contains("plainto_tsquery"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_search_operator_with_logical_and() {
        // Test complex filter combining search with other operators
        let filter = Filter::Logical {
            operator: LogicalOperator::And,
            filters: vec![
                Filter::Field(FieldFilter {
                    field: "provider".to_string(),
                    operator: FilterOperator::Eq,
                    value: FilterValue::String("openai".to_string()),
                }),
                Filter::Field(FieldFilter {
                    field: "input_text".to_string(),
                    operator: FilterOperator::Search,
                    value: FilterValue::String("generate code".to_string()),
                }),
                Filter::Field(FieldFilter {
                    field: "duration_ms".to_string(),
                    operator: FilterOperator::Gt,
                    value: FilterValue::Int(1000),
                }),
            ],
        };

        assert!(filter.validate().is_ok());

        let mut param_index = 1;
        let result = filter.to_sql(&mut param_index);
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();

        // Should contain all three conditions joined with AND
        assert!(sql.contains("AND"));
        assert!(sql.contains("provider = "));
        assert!(sql.contains("input_text_search @@"));
        assert!(sql.contains("duration_ms >"));
        assert_eq!(params.len(), 3);
    }
}
