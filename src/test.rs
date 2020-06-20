use super::*;

use value::{Value, EntityId};
use component_array::{ComponentArray, ComponentRef, ComponentMut};

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
    assert!(decode_value(b"\x85test").is_err());
    assert!(decode_value(b"\x97foobar").is_err());
    assert!(decode_value(b"\xa6\x00\x00\x00").is_err());
    assert!(decode_value(b"\xa7\x00\x00\x00\x00\x00\x00\x00").is_err());
    assert!(decode_value(b"\xa8").is_err());
    assert!(decode_value(b"\xa9\x00").is_err());
    assert!(decode_value(b"\xaa\x00\x00\x00").is_err());
    assert!(decode_value(b"\xab\x00\x00\x00\x00\x00\x00\x00").is_err());
    assert!(decode_value(b"\xad").is_err());
    assert!(decode_value(b"\xad\xad\xad\xad").is_err());

    // 2. invalid byte values
    for byte in 0xb2 .. 0xc0 {
        assert!(decode_value(&[byte]).is_err());
    }
}

fn decode_component_array(b: &[u8]) -> Result<ComponentArray, decode::Error> {
    decode::State::new(b).decode_component_array()
}

fn encode_component_array(array: &ComponentArray) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode::State::new(&mut encoded).encode_component_array(array).unwrap();
    encoded
}

fn check_component_array_round_trip(b: &[u8]) {
    let array = decode_component_array(b).unwrap();
    let encoded = encode_component_array(&array);
    assert_eq!(encoded.as_slice(), b);
}

#[test]
fn component_array_encoding() {
    // error: malformed header
    assert!(decode_component_array(b"").is_err());
    assert!(decode_component_array(b"COMPONENT").is_err());
    assert!(decode_component_array(b"COMPONENT foo 0").is_err());
    assert!(decode_component_array(b"COMPONENT foo 0 0").is_err());
    assert!(decode_component_array(b"TNENOPMOC foo 0 0\n").is_err());

    // ok: properly formed header
    {
        let empty_array = decode_component_array(b"COMPONENT foo 31415 0\n").unwrap();
        assert!(empty_array.is_empty());
        assert_eq!(empty_array.name(), "foo");
        assert_eq!(empty_array.id(), 31415);

        // error: id too large
        assert!(decode_component_array(b"COMPONENT foo 65535 0\n").is_ok());
        assert!(decode_component_array(b"COMPONENT foo 65536 0\n").is_err());

        // ok: header with components
        let scheme = decode_component_array(b"COMPONENT foo 0 0 a b c d e f\n").unwrap();
        assert_eq!(scheme.scheme(), &[
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
            "f".to_string(),
        ]);
    }

    // err: header with unicode
    assert!(decode_component_array(b"COMPONENT \xc1\xa1foo 0 0\n").is_err());

    // ok: header with symbols
    assert_eq!(decode_component_array(b"COMPONENT foo! 0 0\n").unwrap().name(), "foo!");

    // err: too few values
    assert!(decode_component_array(
        b"COMPONENT point 0 5 x y\n\
          \x00\x01\x02\x03\x04\x05\x06\x07\x08"
    ).is_err());

    // ok: correct number of values
    {
        let array = decode_component_array(
            b"COMPONENT point 21718 3 x y\n\
              \xa9\x12\x34\xa9\x23\x45\
              \xa9\x34\x56\xa9\x45\x67\
              \xa9\x56\x78\xa9\x67\x89"
        ).unwrap();

        assert_eq!(array.name(), "point");
        assert_eq!(array.id(), 21718);
        assert_eq!(array.scheme(), &["x".to_string(), "y".to_string()]);

        let comp_0 = array.get(0).unwrap();
        assert_eq!(comp_0.field("x"), Some(&Value::Int(0x1234)));
        assert_eq!(comp_0.field("y"), Some(&Value::Int(0x2345)));

        let comp_1 = array.get(1).unwrap();
        assert_eq!(comp_1.field("x"), Some(&Value::Int(0x3456)));
        assert_eq!(comp_1.field("y"), Some(&Value::Int(0x4567)));

        let comp_2 = array.get(2).unwrap();
        assert_eq!(comp_2.field("x"), Some(&Value::Int(0x5678)));
        assert_eq!(comp_2.field("y"), Some(&Value::Int(0x6789)));
    }

    // ok: mutating a component
    {
        let mut bytes = Vec::new();
        const N: u8 = 100;
        for i in (0..N).map(|i| i.wrapping_mul(i)) {
            bytes.push(i);
        }

        let mut array = decode_component_array(
            b"COMPONENT bytes 0 1 %\n\xac"
        ).unwrap();

        let mut component = array.get_mut(0).unwrap();
        match component.field_mut("%").unwrap() {
            Value::Maybe(m) => {
                assert_eq!(*m, None);
                *m = Some(Box::new(Value::Bytes(bytes.clone())));
            }
            _ => panic!(),
        }

        let mut encoded = Vec::new();
        encoded.extend_from_slice(b"COMPONENT bytes 0 1 %\n\xad\xa0");
        encoded.push(N);
        encoded.extend_from_slice(&bytes);
        assert_eq!(encode_component_array(&array), encoded);
    }

    // ensure that various other things round-trip correctly
    check_component_array_round_trip(b"COMPONENT 2 1 0 1 2\n");
    check_component_array_round_trip(b"COMPONENT foo\x00bar 11111 1 foo bar\n\x01\x02");
}
