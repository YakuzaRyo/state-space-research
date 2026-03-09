//! 数据仓库分层架构的类型安全实现
//! 方向: data_warehouse_analogy
//! 时间: 2026-03-09 12:07
//! 核心思想: 使用Rust类型状态模式实现ETL的硬性边界

use std::collections::HashMap;
use std::marker::PhantomData;

// ============================================================================
// 层标记类型（Layer Markers）
// ============================================================================

/// ODS层 - 贴源层（原始数据）
pub struct ODS;
/// DWD层 - 明细层（清洗后数据）
pub struct DWD;
/// DIM层 - 维度层（主数据）
pub struct DIM;
/// DWS层 - 汇总层（轻度聚合）
pub struct DWS;
/// ADS层 - 应用层（数据超市）
pub struct ADS;

// ============================================================================
// 数据容器（使用PhantomData标记当前层）
// ============================================================================

/// 分层数据表，L表示当前层，T表示数据类型
pub struct DataTable<L, T> {
    name: String,
    data: Vec<T>,
    schema_version: u64,
    _marker: PhantomData<L>,
}

/// 数据质量检查结果
#[derive(Debug, Clone)]
pub struct QualityReport {
    null_count: HashMap<String, usize>,
    duplicate_count: usize,
    format_errors: Vec<String>,
}

/// ETL转换错误
#[derive(Debug)]
pub enum ETLError {
    SchemaMismatch { field: String, expected: String, got: String },
    QualityCheckFailed { table: String, report: QualityReport },
    AggregationError { reason: String },
    BusinessRuleViolation { rule: String },
}

// ============================================================================
// ODS层 - 原始数据（贴源层）
// ============================================================================

/// ODS层数据行 - 保持原始格式，仅做Schema校验
#[derive(Debug, Clone)]
pub struct ODSRow {
    pub raw_data: HashMap<String, String>,
    pub source_system: String,
    pub extract_time: String,
}

/// ODS层Schema约束
pub struct ODSSchema {
    required_fields: Vec<String>,
    field_types: HashMap<String, FieldType>,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Integer,
    Decimal,
    Timestamp,
}

impl ODSSchema {
    /// 验证数据符合Schema（ODS层的硬性边界）
    pub fn validate(&self, row: &ODSRow) -> Result<(), ETLError> {
        // 检查必填字段
        for field in &self.required_fields {
            if !row.raw_data.contains_key(field) {
                return Err(ETLError::SchemaMismatch {
                    field: field.clone(),
                    expected: "present".to_string(),
                    got: "missing".to_string(),
                });
            }
        }
        
        // 检查字段类型
        for (field, expected_type) in &self.field_types {
            if let Some(value) = row.raw_data.get(field) {
                if !self.type_check(value, expected_type) {
                    return Err(ETLError::SchemaMismatch {
                        field: field.clone(),
                        expected: format!("{:?}", expected_type),
                        got: value.clone(),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    fn type_check(&self, value: &str, field_type: &FieldType) -> bool {
        match field_type {
            FieldType::Integer => value.parse::<i64>().is_ok(),
            FieldType::Decimal => value.parse::<f64>().is_ok(),
            FieldType::Timestamp => {
                // 简化的时间戳格式检查
                value.len() == 19 && value.contains('-') && value.contains(':')
            },
            FieldType::String => true,
        }
    }
}

impl DataTable<ODS, ODSRow> {
    pub fn new(name: &str, schema_version: u64) -> Self {
        DataTable {
            name: name.to_string(),
            data: Vec::new(),
            schema_version,
            _marker: PhantomData,
        }
    }
    
    pub fn insert(&mut self, row: ODSRow) {
        self.data.push(row);
    }
    
    /// ODS → DWD 转换（ETL清洗）
    /// 这是关键的状态转换，必须通过数据质量检查
    pub fn into_dwd(
        self,
        schema: &ODSSchema,
        quality_rules: &DataQualityRules,
    ) -> Result<DataTable<DWD, DWDRow>, ETLError> {
        let mut cleaned_data = Vec::new();
        let mut quality_report = QualityReport {
            null_count: HashMap::new(),
            duplicate_count: 0,
            format_errors: Vec::new(),
        };
        
        // Step 1: Schema校验（Syntax层约束）
        for row in &self.data {
            schema.validate(row)?;
        }
        
        // Step 2: 数据清洗（Semantic层约束）
        let mut seen_keys = std::collections::HashSet::new();
        for row in self.data {
            // 去重检查
            let dedup_key = format!("{}:{}", 
                row.raw_data.get("order_id").unwrap_or(&"".to_string()),
                row.raw_data.get("user_id").unwrap_or(&"".to_string())
            );
            if seen_keys.contains(&dedup_key) {
                quality_report.duplicate_count += 1;
                continue;
            }
            seen_keys.insert(dedup_key);
            
            // 空值检查
            for (field, value) in &row.raw_data {
                if value.is_empty() {
                    *quality_report.null_count.entry(field.clone()).or_insert(0) += 1;
                }
            }
            
            // 转换为DWD格式（类型安全转换）
            let dwd_row = DWDRow::from_ods(row)?;
            cleaned_data.push(dwd_row);
        }
        
        // Step 3: 数据质量评估
        if !quality_rules.check(&quality_report) {
            return Err(ETLError::QualityCheckFailed {
                table: self.name.clone(),
                report: quality_report,
            });
        }
        
        Ok(DataTable {
            name: format!("dwd_{}", self.name),
            data: cleaned_data,
            schema_version: self.schema_version,
            _marker: PhantomData,
        })
    }
}

// ============================================================================
// DWD层 - 明细数据层（清洗后的数据）
// ============================================================================

/// DWD层数据行 - 强类型、已清洗
#[derive(Debug, Clone)]
pub struct DWDRow {
    pub order_id: String,
    pub user_id: String,
    pub amount: f64,
    pub create_time: String,
    pub status: OrderStatus,
    pub _provenance: DataProvenance, // 数据血缘
}

#[derive(Debug, Clone)]
pub enum OrderStatus {
    Created,
    Paid,
    Shipped,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DataProvenance {
    pub source_table: String,
    pub extract_time: String,
    pub transform_version: String,
}

impl DWDRow {
    fn from_ods(ods: ODSRow) -> Result<Self, ETLError> {
        // 类型安全转换（Semantic层约束）
        let amount = ods.raw_data.get("amount")
            .and_then(|v| v.parse::<f64>().ok())
            .ok_or_else(|| ETLError::SchemaMismatch {
                field: "amount".to_string(),
                expected: "decimal".to_string(),
                got: ods.raw_data.get("amount").cloned().unwrap_or_default(),
            })?;
        
        Ok(DWDRow {
            order_id: ods.raw_data.get("order_id").cloned().unwrap_or_default(),
            user_id: ods.raw_data.get("user_id").cloned().unwrap_or_default(),
            amount,
            create_time: ods.raw_data.get("create_time").cloned().unwrap_or_default(),
            status: OrderStatus::Created, // 默认值，实际应从源系统映射
            _provenance: DataProvenance {
                source_table: ods.source_system,
                extract_time: ods.extract_time,
                transform_version: "1.0.0".to_string(),
            },
        })
    }
}

/// 数据质量规则（Semantic层的不变量）
pub struct DataQualityRules {
    max_null_ratio: f64,      // 最大空值比例
    max_duplicate_ratio: f64, // 最大重复比例
}

impl DataQualityRules {
    fn check(&self, report: &QualityReport) -> bool {
        let total_rows = report.null_count.values().sum::<usize>() + 1; // 避免除零
        
        // 检查空值率
        for (_, count) in &report.null_count {
            let ratio = *count as f64 / total_rows as f64;
            if ratio > self.max_null_ratio {
                return false;
            }
        }
        
        // 检查重复率
        let dup_ratio = report.duplicate_count as f64 / total_rows as f64;
        if dup_ratio > self.max_duplicate_ratio {
            return false;
        }
        
        true
    }
}

impl DataTable<DWD, DWDRow> {
    /// DWD → DWS 转换（轻度聚合）
    /// Pattern层：应用聚合设计模式
    pub fn into_dws(
        self,
        aggregator: impl Fn(&[DWDRow]) -> Vec<DWSRow>,
    ) -> Result<DataTable<DWS, DWSRow>, ETLError> {
        let aggregated = aggregator(&self.data);
        
        Ok(DataTable {
            name: format!("dws_{}", self.name),
            data: aggregated,
            schema_version: self.schema_version,
            _marker: PhantomData,
        })
    }
}

// ============================================================================
// DWS层 - 汇总数据层（轻度聚合）
// ============================================================================

/// DWS层数据行 - 主题聚合结果
#[derive(Debug, Clone)]
pub struct DWSRow {
    pub user_id: String,
    pub stat_date: String,
    pub total_amount: f64,
    pub order_count: u32,
    pub avg_amount: f64,
}

/// 聚合函数库（Pattern层复用）
pub struct AggregationPatterns;

impl AggregationPatterns {
    /// 按用户+日期聚合（经典DWS模式）
    pub fn user_daily_summary(dwd_rows: &[DWDRow]) -> Vec<DWSRow> {
        use std::collections::HashMap;
        
        let mut groups: HashMap<(String, String), (f64, u32)> = HashMap::new();
        
        for row in dwd_rows {
            let date = &row.create_time[..10]; // 取日期部分
            let key = (row.user_id.clone(), date.to_string());
            
            let (total, count) = groups.entry(key).or_insert((0.0, 0));
            *total += row.amount;
            *count += 1;
        }
        
        groups.into_iter()
            .map(|((user_id, stat_date), (total_amount, order_count))| {
                DWSRow {
                    user_id,
                    stat_date,
                    total_amount,
                    order_count,
                    avg_amount: total_amount / order_count as f64,
                }
            })
            .collect()
    }
}

impl DataTable<DWS, DWSRow> {
    /// DWS → ADS 转换（场景化加工）
    /// Domain层：面向业务场景的约束
    pub fn into_ads(
        self,
        business_logic: impl Fn(&[DWSRow]) -> Result<Vec<ADSRow>, String>,
    ) -> Result<DataTable<ADS, ADSRow>, ETLError> {
        let ads_data = business_logic(&self.data)
            .map_err(|e| ETLError::BusinessRuleViolation { rule: e })?;
        
        Ok(DataTable {
            name: format!("ads_{}", self.name),
            data: ads_data,
            schema_version: self.schema_version,
            _marker: PhantomData,
        })
    }
}

// ============================================================================
// ADS层 - 应用数据层（数据超市）
// ============================================================================

/// ADS层数据行 - 场景化结果
#[derive(Debug, Clone)]
pub struct ADSRow {
    pub user_id: String,
    pub user_segment: String, // 用户分层：高价值/活跃/沉默
    pub ltv_7d: f64,          // 7日生命周期价值
    pub ltv_30d: f64,
    pub risk_score: u8,       // 风险评分
}

/// 业务场景规则（Domain层约束）
pub struct BusinessScenarios;

impl BusinessScenarios {
    /// 用户价值分层场景（数据超市典型应用）
    pub fn user_segmentation(dws_rows: &[DWSRow]) -> Result<Vec<ADSRow>, String> {
        let mut results = Vec::new();
        
        for row in dws_rows {
            // 业务规则：根据消费金额分层
            let segment = if row.total_amount > 10000.0 {
                "高价值用户"
            } else if row.total_amount > 1000.0 {
                "活跃用户"
            } else {
                "普通用户"
            };
            
            // 风险评分逻辑（简化示例）
            let risk_score = if row.order_count > 100 && row.avg_amount < 10.0 {
                80 // 高频低额 = 潜在刷单风险
            } else {
                20
            };
            
            results.push(ADSRow {
                user_id: row.user_id.clone(),
                user_segment: segment.to_string(),
                ltv_7d: row.total_amount * 0.3, // 简化计算
                ltv_30d: row.total_amount * 1.2,
                risk_score,
            });
        }
        
        Ok(results)
    }
}

// ============================================================================
// 类型级血缘追踪（编译期可验证）
// ============================================================================

/// 数据血缘追踪器
pub struct DataLineage<From, To> {
    from_layer: PhantomData<From>,
    to_layer: PhantomData<To>,
    transformations: Vec<String>,
}

impl<From, To> DataLineage<From, To> {
    pub fn new() -> Self {
        DataLineage {
            from_layer: PhantomData,
            to_layer: PhantomData,
            transformations: Vec::new(),
        }
    }
    
    pub fn add_step(mut self, description: &str) -> Self {
        self.transformations.push(description.to_string());
        self
    }
    
    pub fn trace(&self) -> String {
        self.transformations.join(" → ")
    }
}

// ============================================================================
// 使用示例
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_etl_pipeline() {
        // 1. 创建ODS层原始数据
        let mut ods_table = DataTable::<ODS, ODSRow>::new("orders", 1);
        
        ods_table.insert(ODSRow {
            raw_data: [
                ("order_id".to_string(), "ORD001".to_string()),
                ("user_id".to_string(), "USR001".to_string()),
                ("amount".to_string(), "199.99".to_string()),
                ("create_time".to_string(), "2026-03-09 12:00:00".to_string()),
            ].into_iter().collect(),
            source_system: "ecommerce_db".to_string(),
            extract_time: "2026-03-09T12:00:00Z".to_string(),
        });
        
        // 2. 定义Schema约束（ODS层硬性边界）
        let schema = ODSSchema {
            required_fields: vec!["order_id", "user_id", "amount"],
            field_types: [
                ("amount".to_string(), FieldType::Decimal),
                ("create_time".to_string(), FieldType::Timestamp),
            ].into_iter().collect(),
        };
        
        // 3. 定义数据质量规则（DWD层硬性边界）
        let quality_rules = DataQualityRules {
            max_null_ratio: 0.1,
            max_duplicate_ratio: 0.05,
        };
        
        // 4. 执行ETL管道（类型状态转换）
        let dwd_table = ods_table
            .into_dwd(&schema, &quality_rules)
            .expect("ODS→DWD转换失败");
        
        let dws_table = dwd_table
            .into_dws(AggregationPatterns::user_daily_summary)
            .expect("DWD→DWS转换失败");
        
        let ads_table = dws_table
            .into_ads(BusinessScenarios::user_segmentation)
            .expect("DWS→ADS转换失败");
        
        // 5. 验证结果
        assert_eq!(ads_table.name, "ads_dwd_orders");
        println!("✅ ETL管道执行成功！");
        println!("   最终数据行数: {}", ads_table.data.len());
        
        // 6. 数据血缘追踪
        let lineage = DataLineage::<ODS, ADS>::new()
            .add_step("抽取: ecommerce_db.orders")
            .add_step("清洗: Schema校验+去重+空值处理")
            .add_step("转换: 按user_id+date聚合")
            .add_step("应用: 用户价值分层");
        
        println!("   数据血缘: {}", lineage.trace());
    }
}
