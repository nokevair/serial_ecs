mod error;
use error::{Error, Result};

mod parse;
pub mod value;

#[cfg(test)]
mod tests {
    use super::*;
    use value::Value;

    fn parse_val(b: &[u8]) -> Result<Value> {
        parse::State::new(b).parse_value()
    }

    #[test]
    fn basic_vals() {
        // 7-bit numeric literals
        for b in 0..128 {
            assert_eq!(parse_val(&[b]).unwrap(), Value::Int(b as i64));
        }

        // 4-bit string literals
        assert_eq!(parse_val(b"\x80").unwrap(), Value::Bytes(Vec::new()));
        assert_eq!(parse_val(b"\x84test").unwrap(), Value::Bytes(b"test".to_vec()));
        assert!(parse_val(b"\x85test").is_err());

        // 4-bit array literals
        assert_eq!(parse_val(b"\x90").unwrap(), Value::Array(Vec::new()));
        assert_eq!(parse_val(b"\x94\x01\x02\x03\x04").unwrap(), Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]));
        assert_eq!(
            parse_val(b"\x92\x92\x01\x02\x92\x03\x04").unwrap(),
            Value::Array(vec![
                Value::Array(vec![
                    Value::Int(1),
                    Value::Int(2),
                ]),
                Value::Array(vec![
                    Value::Int(3),
                    Value::Int(4),
                ]),
            ]
        ));

        // 8-bit array literals
        assert_eq!(parse_val(b"\xa2\x00").unwrap(), Value::Array(Vec::new()));
        {
            const N: u8 = 5;
            let mut encoded = Vec::new();
            let mut expected_array = Vec::new();

            encoded.push(0xa2);
            encoded.push(N);
            for val in (0..N).map(|i| i.wrapping_mul(i)) {
                if val < 128 {
                    encoded.push(val);
                } else {
                    encoded.push(0xa9);
                    encoded.push(0x00);
                    encoded.push(val);
                }
                expected_array.push(Value::Int(val as i64));
            }

            assert_eq!(parse_val(&encoded).unwrap(), Value::Array(expected_array));
        }

        // booleans
        assert_eq!(
            parse_val(b"\x92\xa4\xa5").unwrap(),
            Value::Array(vec![Value::Bool(false), Value::Bool(true)]),
        );

        // floats
        {
            use std::f32::consts::PI as PI_F32;
            use std::f64::consts::PI as PI_F64;

            let mut encoded = Vec::new();
            encoded.push(0x92);
            encoded.push(0xa6);
            encoded.extend_from_slice(&PI_F32.to_be_bytes());
            encoded.push(0xa7);
            encoded.extend_from_slice(&PI_F64.to_be_bytes());

            assert_eq!(
                parse_val(&encoded).unwrap(),
                Value::Array(vec![Value::Float(PI_F32 as f64), Value::Float(PI_F64)]),
            )
        }
    }
}
