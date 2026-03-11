//! JSON Schema验证器 - 简化实现
//!
//! 支持:
//! - 基本类型验证 (string, integer, number, boolean, array, object)
//! - 必需字段检查
//! - 嵌套对象验证
//! - 数组元素验证

use serde_json::Value;
use crate::GrammarError;

/// JSON Schema验证器
#[derive(Debug)]
pub struct JsonSchemaValidator {
    schema: Value,
}

impl JsonSchemaValidator {
    /// 从JSON字符串创建验证器
    pub fn from_schema(schema_str: &str) -> Result<Self, GrammarError> {
        let schema: Value = serde_json::from_str(schema_str)
            .map_err(|e| GrammarError::InvalidSyntax(format!("Invalid JSON schema: {}", e)))?;

        Ok(Self { schema })
    }

    /// 验证JSON字符串
    pub fn validate(&self, json_str: &str) -> Result<(), GrammarError> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| GrammarError::InvalidSyntax(format!("Invalid JSON: {}", e)))?;

        self.validate_value(&value, &self.schema)
    }

    /// 验证值是否符合schema
    fn validate_value(&self, value: &Value, schema: &Value) -> Result<(), GrammarError> {
        // 处理schema引用
        let schema = if let Some(ref_val) = schema.get("$ref") {
            // 简化: 忽略$ref解析
            schema
        } else {
            schema
        };

        // 类型验证
        if let Some(type_val) = schema.get("type") {
            self.validate_type(value, type_val)?;
        }

        // 对象验证
        if let Some(properties) = schema.get("properties") {
            if let Value::Object(props) = properties {
                self.validate_properties(value, props, schema.get("required"))?;
            }
        }

        // 数组验证
        if let Some(items) = schema.get("items") {
            if let Value::Array(arr) = value {
                for (i, item) in arr.iter().enumerate() {
                    self.validate_value(item, items)
                        .map_err(|e| GrammarError::SchemaViolation(
                            format!("Item {}: {}", i, e)
                        ))?;
                }
            }
        }

        // 数值范围验证
        if let Value::Number(n) = value {
            if let Some(min) = schema.get("minimum") {
                if let Some(min_val) = min.as_f64() {
                    if n.as_f64().unwrap_or(0.0) < min_val {
                        return Err(GrammarError::SchemaViolation(
                            format!("Value {} is less than minimum {}", n, min_val)
                        ));
                    }
                }
            }

            if let Some(max) = schema.get("maximum") {
                if let Some(max_val) = max.as_f64() {
                    if n.as_f64().unwrap_or(0.0) > max_val {
                        return Err(GrammarError::SchemaViolation(
                            format!("Value {} is greater than maximum {}", n, max_val)
                        ));
                    }
                }
            }
        }

        // 字符串长度验证
        if let Value::String(s) = value {
            if let Some(min_len) = schema.get("minLength") {
                if let Some(min) = min_len.as_u64() {
                    if s.len() < min as usize {
                        return Err(GrammarError::SchemaViolation(
                            format!("String length {} is less than minLength {}", s.len(), min)
                        ));
                    }
                }
            }

            if let Some(max_len) = schema.get("maxLength") {
                if let Some(max) = max_len.as_u64() {
                    if s.len() > max as usize {
                        return Err(GrammarError::SchemaViolation(
                            format!("String length {} is greater than maxLength {}", s.len(), max)
                        ));
                    }
                }
            }
        }

        // 枚举验证
        if let Some(enum_vals) = schema.get("enum") {
            if let Value::Array(vals) = enum_vals {
                if !vals.contains(value) {
                    return Err(GrammarError::SchemaViolation(
                        format!("Value {:?} is not in enum {:?}", value, vals)
                    ));
                }
            }
        }

        Ok(())
    }

    /// 验证类型
    fn validate_type(&self, value: &Value, type_val: &Value) -> Result<(), GrammarError> {
        let expected_type = type_val.as_str()
            .ok_or_else(|| GrammarError::InvalidSyntax("Invalid type in schema".to_string()))?;

        let matches = match expected_type {
            "string" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => return Err(GrammarError::InvalidSyntax(
                format!("Unknown type: {}", expected_type)
            )),
        };

        if matches {
            Ok(())
        } else {
            Err(GrammarError::SchemaViolation(
                format!("Expected type {}, got {:?}", expected_type, value)
            ))
        }
    }

    /// 验证对象属性
    fn validate_properties(
        &self,
        value: &Value,
        properties: &serde_json::Map<String, Value>,
        required: Option<&Value>,
    ) -> Result<(), GrammarError> {
        let obj = match value {
            Value::Object(o) => o,
            _ => return Err(GrammarError::SchemaViolation(
                "Expected object".to_string()
            )),
        };

        // 验证必需字段
        if let Some(req) = required {
            if let Value::Array(req_fields) = req {
                for field in req_fields {
                    if let Some(field_name) = field.as_str() {
                        if !obj.contains_key(field_name) {
                            return Err(GrammarError::SchemaViolation(
                                format!("Missing required field: {}", field_name)
                            ));
                        }
                    }
                }
            }
        }

        // 验证每个属性
        for (prop_name, prop_schema) in properties {
            if let Some(prop_value) = obj.get(prop_name) {
                self.validate_value(prop_value, prop_schema)
                    .map_err(|e| GrammarError::SchemaViolation(
                        format!("Field '{}': {}", prop_name, e)
                    ))?;
            }
        }

        Ok(())
    }
}

/// 从JSON Schema生成EBNF语法
pub fn schema_to_ebnf(schema: &Value) -> String {
    let mut grammar = String::new();

    grammar.push_str("root ::= object\n\n");
    grammar.push_str("object ::= \"{\" pair_list \"}\"\n");
    grammar.push_str("pair_list ::= pair (\",\" pair)* | \"\"\n");
    grammar.push_str("pair ::= string \":\" value\n\n");
    grammar.push_str("value ::= object | array | string | number | \"true\" | \"false\" | \"null\"\n");
    grammar.push_str("array ::= \"[\" value_list \"]\"\n");
    grammar.push_str("value_list ::= value (\",\" value)* | \"\"\n\n");
    grammar.push_str("string ::= \"\\\"\" char* \"\\\"\"\n");
    grammar.push_str("char ::= [^\"\\\\] | \"\\\\\" esc_char\n");
    grammar.push_str("esc_char ::= [\"\\\\/bfnrt] | \"u\" hex_digit{4}\n");
    grammar.push_str("hex_digit ::= [0-9a-fA-F]\n\n");
    grammar.push_str("number ::= \"-\"? int frac? exp?\n");
    grammar.push_str("int ::= \"0\" | [1-9] [0-9]*\n");
    grammar.push_str("frac ::= \".\" [0-9]+\n");
    grammar.push_str("exp ::= [eE] [+-]? [0-9]+\n");

    grammar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_string() {
        let schema = r#"{"type": "string"}"#;
        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#""hello""#).is_ok());
        assert!(validator.validate(r#"123"#).is_err());
    }

    #[test]
    fn test_validate_object() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        }"#;

        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#"{"name": "Alice", "age": 30}"#).is_ok());
        assert!(validator.validate(r#"{"age": 30}"#).is_err()); // missing required
        assert!(validator.validate(r#"{"name": 123}"#).is_err()); // wrong type
    }

    #[test]
    fn test_validate_array() {
        let schema = r#"{
            "type": "array",
            "items": {"type": "integer"}
        }"#;

        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#"[1, 2, 3]"#).is_ok());
        assert!(validator.validate(r#"[1, "two", 3]"#).is_err());
    }

    #[test]
    fn test_validate_minimum() {
        let schema = r#"{"type": "integer", "minimum": 0}"#;
        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#"5"#).is_ok());
        assert!(validator.validate(r#"0"#).is_ok());
        assert!(validator.validate(r#"-1"#).is_err());
    }

    #[test]
    fn test_schema_to_ebnf() {
        let schema: Value = serde_json::from_str(r#"{"type": "object"}"#).unwrap();
        let ebnf = schema_to_ebnf(&schema);

        assert!(ebnf.contains("root ::= object"));
        assert!(ebnf.contains("object ::="));
        assert!(ebnf.contains("value ::="));
    }
}
