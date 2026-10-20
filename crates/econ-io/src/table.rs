//! A table of plain columns as Arrow IPC bytes, for crates that must not
//! depend on Arrow themselves (the scenario builder). Arrow lives in this
//! crate only (ADR-0005).

use std::sync::Arc;

use arrow_array::{
    ArrayRef, Int64Array, RecordBatch, StringArray, UInt8Array, UInt16Array, UInt32Array,
    UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};

use crate::{SaveError, arrow_err, ipc_bytes};

/// The values of one column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Column {
    /// Small codes.
    U8(Vec<u8>),
    /// Ids.
    U16(Vec<u16>),
    /// Counts and millionths.
    U32(Vec<u32>),
    /// Amounts and counts.
    U64(Vec<u64>),
    /// Signed amounts.
    I64(Vec<i64>),
    /// Text.
    Utf8(Vec<String>),
}

impl Column {
    fn array(self) -> (DataType, ArrayRef) {
        match self {
            Column::U8(v) => (DataType::UInt8, Arc::new(UInt8Array::from(v))),
            Column::U16(v) => (DataType::UInt16, Arc::new(UInt16Array::from(v))),
            Column::U32(v) => (DataType::UInt32, Arc::new(UInt32Array::from(v))),
            Column::U64(v) => (DataType::UInt64, Arc::new(UInt64Array::from(v))),
            Column::I64(v) => (DataType::Int64, Arc::new(Int64Array::from(v))),
            Column::Utf8(v) => (DataType::Utf8, Arc::new(StringArray::from(v))),
        }
    }
}

/// A named column with an optional note in its field metadata (key `unit`,
/// for example "bani, stock"). One key only, so the bytes do not depend on
/// the order of a map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedColumn {
    /// Column name.
    pub name: &'static str,
    /// Values.
    pub values: Column,
    /// The `unit` metadata of the field, if any.
    pub unit: Option<&'static str>,
}

/// The columns as one zstd-compressed Arrow IPC file (one record batch).
///
/// # Errors
/// Columns of different lengths, or an encoding failure.
pub fn ipc_table(columns: Vec<NamedColumn>) -> Result<Vec<u8>, SaveError> {
    let mut fields = Vec::with_capacity(columns.len());
    let mut arrays = Vec::with_capacity(columns.len());
    for c in columns {
        let (data_type, array) = c.values.array();
        let mut field = Field::new(c.name, data_type, false);
        if let Some(unit) = c.unit {
            field = field.with_metadata(
                [("unit".to_string(), unit.to_string())]
                    .into_iter()
                    .collect(),
            );
        }
        fields.push(field);
        arrays.push(array);
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays).map_err(arrow_err)?;
    ipc_bytes(&batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_batch;
    use arrow_array::Array;

    #[test]
    fn a_table_reads_back_with_its_metadata() {
        let bytes = ipc_table(vec![
            NamedColumn {
                name: "id",
                values: Column::Utf8(vec!["a".into(), "b".into()]),
                unit: None,
            },
            NamedColumn {
                name: "deposits",
                values: Column::U64(vec![10, 20]),
                unit: Some("bani, stock"),
            },
        ])
        .unwrap();
        assert_eq!(
            bytes,
            ipc_table(vec![
                NamedColumn {
                    name: "id",
                    values: Column::Utf8(vec!["a".into(), "b".into()]),
                    unit: None,
                },
                NamedColumn {
                    name: "deposits",
                    values: Column::U64(vec![10, 20]),
                    unit: Some("bani, stock"),
                },
            ])
            .unwrap()
        );
        let b = read_batch(bytes).unwrap();
        assert_eq!(b.num_rows(), 2);
        let schema = b.schema();
        let f = schema.field_with_name("deposits").unwrap();
        assert_eq!(
            f.metadata().get("unit").map(String::as_str),
            Some("bani, stock")
        );
        let d = b.column(1).as_any().downcast_ref::<UInt64Array>().unwrap();
        assert_eq!(d.value(1), 20);
    }

    #[test]
    fn columns_of_different_lengths_are_refused() {
        let r = ipc_table(vec![
            NamedColumn {
                name: "a",
                values: Column::U8(vec![1]),
                unit: None,
            },
            NamedColumn {
                name: "b",
                values: Column::U8(vec![1, 2]),
                unit: None,
            },
        ]);
        assert!(r.is_err());
    }
}
