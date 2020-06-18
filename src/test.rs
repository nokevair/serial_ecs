use super::*;
use value::{Value, EntityId};

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

    // ints
    assert_eq!(parse_val(b"\xa8\x7f").unwrap(), Value::Int(0x7f));
    assert_eq!(parse_val(b"\xa8\x80").unwrap(), Value::Int(-0x80));
    assert_eq!(parse_val(b"\xa9\x7f\xff").unwrap(), Value::Int(0x7fff));
    assert_eq!(parse_val(b"\xa9\x80\x00").unwrap(), Value::Int(-0x8000));
    assert_eq!(parse_val(b"\xaa\x7f\xff\xff\xff").unwrap(), Value::Int(0x7fffffff));
    assert_eq!(parse_val(b"\xaa\x80\x00\x00\x00").unwrap(), Value::Int(-0x80000000));
    assert_eq!(
        parse_val(b"\xab\x7f\xff\xff\xff\xff\xff\xff\xff").unwrap(),
        Value::Int(0x7fffffffffffffff),
    );
    assert_eq!(
        parse_val(b"\xab\x80\x00\x00\x00\x00\x00\x00\x00").unwrap(),
        Value::Int(-0x8000000000000000),
    );

    // optionals
    assert_eq!(parse_val(b"\xac").unwrap(), Value::Maybe(None));
    assert_eq!(parse_val(b"\xad\x01").unwrap(),
        Value::Maybe(Some(Box::new(Value::Int(1)))));
    assert_eq!(parse_val(b"\xad\xad\xad\xac").unwrap(),
        Value::Maybe(Some(Box::new(
            Value::Maybe(Some(Box::new(
                Value::Maybe(Some(Box::new(
                    Value::Maybe(None)))))))))));
    
    // entity IDs
    assert_eq!(parse_val(b"\xae\xab").unwrap(),
        Value::EntityId(EntityId::Idx(0xab)));
    assert_eq!(parse_val(b"\xaf\xab\xcd").unwrap(),
        Value::EntityId(EntityId::Idx(0xabcd)));
    assert_eq!(parse_val(b"\xb0\xab\xcd\xef\x01").unwrap(),
        Value::EntityId(EntityId::Idx(0xabcdef01)));
    assert_eq!(parse_val(b"\xb1").unwrap(),
        Value::EntityId(EntityId::Invalid));

    assert_eq!(parse_val(b"\xc0").unwrap(), Value::EntityId(EntityId::Idx(0)));
    assert_eq!(parse_val(b"\xff").unwrap(), Value::EntityId(EntityId::Idx(63)));

    // invalid byte values
    for byte in 0xb2 .. 0xc0 {
        assert!(matches!(
            parse_val(&[byte]),
            Err(Error::BadValueByte(b)) if b == byte
        ));
    }
}
