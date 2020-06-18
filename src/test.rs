use super::*;
use value::{Value, EntityId};

fn decode_value(b: &[u8]) -> Result<Value, decode::Error> {
    decode::State::new(b).decode_value()
}

fn encode_value(val: &Value) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode::State::new(&mut encoded).encode_value(&val).unwrap();
    encoded
}

fn check_value_decode(b: &[u8], val: Value) {
    let decoded = decode_value(b).unwrap();
    assert_eq!(decoded, val);
}

fn check_value_round_trip(b: &[u8], val: Value) {
    let decoded = decode_value(b).unwrap();
    let encoded = encode_value(&val);

    assert_eq!(decoded, val);
    assert_eq!(encoded, b);
}

#[test]
fn value_encoding() {
    // 7-bit numeric literals
    for b in 0..0x80 {
        check_value_round_trip(&[b], Value::Int(b as i64));
    }

    // 4-bit string literals
    check_value_round_trip(b"\x80", Value::Bytes(Vec::new()));
    check_value_round_trip(b"\x84test", Value::Bytes(b"test".to_vec()));

    // 4-bit array literals
    check_value_round_trip(b"\x90", Value::Array(Vec::new()));
    check_value_round_trip(b"\x94\x01\x02\x03\x04", Value::Array(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
        Value::Int(4),
    ]));
    check_value_round_trip(
        b"\x92\x92\x01\x02\x92\x03\x04",
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
    check_value_decode(b"\xa2\x00", Value::Array(Vec::new()));
    {
        const N: u8 = 100;
        let mut encoded = Vec::new();
        let mut expected_array = Vec::new();

        encoded.push(0xa2);
        encoded.push(N);
        for val in (0..N).map(|i| i.wrapping_mul(i)) {
            if val < 0x80 {
                encoded.push(val);
            } else {
                encoded.push(0xa9);
                encoded.push(0x00);
                encoded.push(val);
            }
            expected_array.push(Value::Int(val as i64));
        }

        check_value_round_trip(&encoded, Value::Array(expected_array));
    }

    // booleans
    check_value_round_trip(
        b"\x92\xa4\xa5",
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

        // don't attempt to round-trip, since f64s aren't converted back to f32s.
        check_value_decode(
            &encoded,
            Value::Array(vec![Value::Float(PI_F32 as f64), Value::Float(PI_F64)]),
        )
    }

    // ints
    check_value_decode    (b"\xa8\x7f", Value::Int(0x7f));
    check_value_round_trip(b"\xa8\x80", Value::Int(-0x80));
    check_value_round_trip(b"\xa9\x7f\xff", Value::Int(0x7fff));
    check_value_round_trip(b"\xa9\x80\x00", Value::Int(-0x8000));
    check_value_round_trip(b"\xaa\x7f\xff\xff\xff", Value::Int(0x7fffffff));
    check_value_round_trip(b"\xaa\x80\x00\x00\x00", Value::Int(-0x80000000));
    check_value_round_trip(
        b"\xab\x7f\xff\xff\xff\xff\xff\xff\xff",
        Value::Int(0x7fffffffffffffff),
    );
    check_value_round_trip(
        b"\xab\x80\x00\x00\x00\x00\x00\x00\x00",
        Value::Int(-0x8000000000000000),
    );

    // optionals
    check_value_round_trip(b"\xac", Value::Maybe(None));
    check_value_round_trip(b"\xad\x01",
        Value::Maybe(Some(Box::new(Value::Int(1)))));
    check_value_round_trip(b"\xad\xad\xad\xac",
        Value::Maybe(Some(Box::new(
            Value::Maybe(Some(Box::new(
                Value::Maybe(Some(Box::new(
                    Value::Maybe(None)))))))))));
    
    // entity IDs
    check_value_round_trip(b"\xae\xab",
        Value::EntityId(EntityId::Idx(0xab)));
    check_value_round_trip(b"\xaf\xab\xcd",
        Value::EntityId(EntityId::Idx(0xabcd)));
    check_value_round_trip(b"\xb0\xab\xcd\xef\x01",
        Value::EntityId(EntityId::Idx(0xabcdef01)));
    check_value_round_trip(b"\xb1",
        Value::EntityId(EntityId::Invalid));

    check_value_round_trip(b"\xc0", Value::EntityId(EntityId::Idx(0)));
    check_value_round_trip(b"\xff", Value::EntityId(EntityId::Idx(0x3f)));

    // syntax errors:

    // 1. string, array, option, and numeric literals that are too short
    assert!(matches!(decode_value(b"\x85test"), Err(_)));
    assert!(matches!(decode_value(b"\x97foobar"), Err(_)));
    assert!(matches!(decode_value(b"\xa6\x00\x00\x00"), Err(_)));
    assert!(matches!(decode_value(b"\xa7\x00\x00\x00\x00\x00\x00\x00"), Err(_)));
    assert!(matches!(decode_value(b"\xa8"), Err(_)));
    assert!(matches!(decode_value(b"\xa9\x00"), Err(_)));
    assert!(matches!(decode_value(b"\xaa\x00\x00\x00"), Err(_)));
    assert!(matches!(decode_value(b"\xab\x00\x00\x00\x00\x00\x00\x00"), Err(_)));
    assert!(matches!(decode_value(b"\xad"), Err(_)));
    assert!(matches!(decode_value(b"\xad\xad\xad\xad"), Err(_)));

    // 2. invalid byte values
    for byte in 0xb2 .. 0xc0 {
        assert!(matches!(
            decode_value(&[byte]),
            Err(decode::Error::BadValueByte(b)) if b == byte
        ));
    }
}
