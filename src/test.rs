use super::*;

use value::{Value, EntityId};
use component::{ComponentArray, GlobalComponent};
use entity::{ComponentIdx, EntityData};

/// Return an arbitrary byte vector for testing purposes, as well as its length.
fn get_bytes() -> (u8, Vec<u8>) {
    const N: u8 = 100;
    (N, (0..N).map(|i| i.wrapping_mul(i)).collect())
}

fn decode_value(b: &[u8]) -> Result<Value, decode::Error> {
    decode::State::new(b).decode_value()
}

fn encode_value(val: &Value) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode::State::new(&mut encoded).encode_value(&val, &mut |_| {}).unwrap();
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
        let (n, vals) = get_bytes();
        let mut encoded = Vec::new();
        let mut expected_array = Vec::new();

        encoded.push(0xa2);
        encoded.push(n);
        for val in vals {
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

        check_value_round_trip(
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
    assert_eq!(encoded, b);
}

#[test]
fn component_array_encoding() {
    // error: malformed header
    assert!(decode_component_array(b"").is_err());
    assert!(decode_component_array(b"COMPONENT").is_err());
    assert!(decode_component_array(b"COMPONENT foo 0").is_err());
    assert!(decode_component_array(b"COMPONENT foo 0 0").is_err());
    assert!(decode_component_array(b"TNENOPMOC foo 0 0\n").is_err());

    // error: duplicate fields
    assert!(decode_component_array(b"COMPONENT foo 0 0 a a\n").is_err());
    assert!(decode_component_array(b"COMPONENT foo 0 0 a b a\n").is_err());
    assert!(decode_component_array(b"COMPONENT foo 0 0 a b c d a f\n").is_err());

    // ok: properly formed header
    {
        let mut empty_array = decode_component_array(b"COMPONENT foo 31415 0\n").unwrap();
        assert!(empty_array.is_marker());
        assert_eq!(empty_array.name(), "foo");
        assert_eq!(empty_array.id(), 31415);

        // the only valid index for marker components is zero
        assert!(empty_array.get(0).is_some());
        assert!(empty_array.get(1).is_none());
        assert!(empty_array.get_mut(0).is_some());
        assert!(empty_array.get_mut(1).is_none());
    }

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

    // error: header with unicode
    assert!(decode_component_array(b"COMPONENT \xc1\xa1foo 0 0\n").is_err());

    // ok: header with symbols
    assert_eq!(decode_component_array(b"COMPONENT foo! 0 0\n").unwrap().name(), "foo!");

    // error: too few values
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
        let (n, bytes) = get_bytes();

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
        encoded.push(n);
        encoded.extend_from_slice(&bytes);
        assert_eq!(encode_component_array(&array), encoded);
    }

    // ensure that various other things round-trip correctly
    check_component_array_round_trip(b"COMPONENT 2 1 0 1 2\n");
    check_component_array_round_trip(b"COMPONENT foo\x00bar 11111 1 foo bar\n\x01\x02");
}

fn decode_global_component(b: &[u8]) -> Result<GlobalComponent, decode::Error> {
    decode::State::new(b).decode_global_component()
}

fn encode_global_component(global: &GlobalComponent) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode::State::new(&mut encoded).encode_global_component(global).unwrap();
    encoded
}

#[test]
fn global_component_encoding() {
    // error: malformed header
    assert!(decode_global_component(b"").is_err());
    assert!(decode_global_component(b"GLOBAL").is_err());
    assert!(decode_global_component(b"LABOLG\n").is_err());
    
    // ok: properly formed header
    {
        let component = decode_global_component(b"GLOBAL\n").unwrap();
        assert!(component.is_empty());
    }

    // error: duplicate fields
    assert!(decode_global_component(b"GLOBAL a a\n").is_err());
    assert!(decode_global_component(b"GLOBAL a b a\n").is_err());
    assert!(decode_global_component(b"GLOBAL a b c d a f\n").is_err());

    // error: too few values
    assert!(decode_global_component(b"GLOBAL a\n").is_err());
    assert!(decode_global_component(b"GLOBAL a b\n\x00").is_err());

    // ok: correct number of values
    {
        let global = decode_global_component(b"GLOBAL x y z\n\x12\x34\x56").unwrap();
        assert!(!global.is_empty());
        assert_eq!(global.scheme(), &[
            "x".to_string(),
            "y".to_string(),
            "z".to_string(),
        ]);
        assert_eq!(global.field_idx("x"), Some(0));
        assert_eq!(global.field_idx("y"), Some(1));
        assert_eq!(global.field_idx("z"), Some(2));

        let component = global.get();
        assert_eq!(component.field("x"), Some(&Value::Int(0x12)));
        assert_eq!(component.field("y"), Some(&Value::Int(0x34)));
        assert_eq!(component.field("z"), Some(&Value::Int(0x56)));
    }

    // ok: mutating a component
    {
        let (n, bytes) = get_bytes();

        let mut global = decode_global_component(b"GLOBAL bytes\n\xac").unwrap();
        let mut component = global.get_mut();
        match component.field_mut("bytes").unwrap() {
            Value::Maybe(m) => {
                assert_eq!(*m, None);
                *m = Some(Box::new(Value::Bytes(bytes.clone())));
            }
            _ => panic!()
        }

        let mut encoded = Vec::new();
        encoded.extend_from_slice(b"GLOBAL bytes\n\xad\xa0");
        encoded.push(n);
        encoded.extend_from_slice(&bytes);
        assert_eq!(encode_global_component(&global), encoded);
    }
}

fn decode_component_idx(b: &[u8]) -> Result<ComponentIdx, decode::Error> {
    decode::State::new(b).decode_component_idx()
}

fn encode_component_idx(idx: ComponentIdx) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode::State::new(&mut encoded).encode_component_idx(idx).unwrap();
    encoded
}

fn check_component_idx_round_trip(b: &[u8], id: u16, idx: u32) {
    let comp_idx = decode_component_idx(b).unwrap();
    let encoded = encode_component_idx(comp_idx);
    assert_eq!(encoded, b);
    assert_eq!(comp_idx.id, id);
    assert_eq!(comp_idx.idx, idx);
}

#[test]
fn component_idx_encoding() {
    // 6-bit id
    for &id in &[0, 0xf, 0x1f, 0x2f, 0x3f] {
        // 0-bit idx
        check_component_idx_round_trip(&[0xc0 + id], id as u16, 0);
        for &idx in &[1, 0x40, 0x7f, 0xbe, 0xfd] {
            // 8-bit idx
            check_component_idx_round_trip(&[id, idx], id as u16, idx as u32);
            // 16-bit idx
            check_component_idx_round_trip(&[0x40 + id, 1, idx], id as u16, idx as u32 + 0x100);
            check_component_idx_round_trip(&[0x40 + id, 16, idx], id as u16, idx as u32 + 0x1000);
        }
    }

    // 8-bit id
    for &id in &[0x40, 0x7f, 0xbe, 0xfd] {
        // 0-bit idx
        check_component_idx_round_trip(&[0x88, id], id as u16, 0);
        // 8-bit idx
        check_component_idx_round_trip(&[0x80, id, id], id as u16, id as u32);
        // 16-bit idx
        check_component_idx_round_trip(&[0x81, id, id, id], id as u16,
            u32::from_be_bytes([0, 0, id, id]));
        // 24-bit idx
        check_component_idx_round_trip(&[0x82, id, id, id, id], id as u16,
            u32::from_be_bytes([0, id, id, id]));
        // 32-bit idx
        check_component_idx_round_trip(&[0x83, id, id, id, id, id], id as u16,
            u32::from_be_bytes([id, id, id, id]));
    }

    // 16-bit id
    for &id in &[0x100, 0x200, 0x1234, 0xffff] {
        let [id_a, id_b] = u16::to_be_bytes(id);
        // 0-bit idx
        check_component_idx_round_trip(&[0x89, id_a, id_b], id, 0);
        // 8-bit idx
        check_component_idx_round_trip(&[0x84, id_a, id_b, id_a], id, id_a as u32);
        // 16-bit idx
        check_component_idx_round_trip(&[0x85, id_a, id_b, id_a, id_b], id, id as u32);
        // 24-bit idx
        check_component_idx_round_trip(&[0x86, id_a, id_b, id_a, id_b, id_a], id,
            u32::from_be_bytes([0, id_a, id_b, id_a]));
        // 32-bit idx
        check_component_idx_round_trip(&[0x87, id_a, id_b, id_a, id_b, id_a, id_b], id,
            u32::from_be_bytes([id_a, id_b, id_a, id_b]));
    }
}

fn decode_entity_data(b: &[u8]) -> Result<EntityData, decode::Error> {
    decode::State::new(b).decode_entity_data()
}

fn encode_entity_data(data: &EntityData) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode::State::new(&mut encoded).encode_entity_data(data).unwrap();
    encoded
}

fn check_entity_data_round_trip(b: &[u8]) -> EntityData {
    let data = decode_entity_data(b).unwrap();
    let encoded = encode_entity_data(&data);
    assert_eq!(encoded, b);
    data
}

#[test]
fn entity_data_encoding() {
    // ok: correct number of component idxs
    assert_eq!(check_entity_data_round_trip(b"\x00").components, Vec::new());
    assert_eq!(check_entity_data_round_trip(b"\x01\x01\x01").components, vec![ComponentIdx {
        id: 1,
        idx: 1,
    }]);

    // error: too few component idxs
    assert!(decode_entity_data(b"\x01").is_err());
    assert!(decode_entity_data(b"\x02\x01\x01").is_err());

    // ok: large number of component idxs
    {
        let mut encoded = Vec::new();
        let mut components = Vec::new();

        let (n, bytes) = get_bytes();
        encoded.push(n);

        for (i, id) in bytes.into_iter().enumerate() {
            let idx = (i % 5) as u8;
            match (id < 0x40, idx == 0) {
                (false, false) => encoded.extend_from_slice(&[0x80, id, idx]),
                (true,  false) => encoded.extend_from_slice(&[id, idx]),
                (false, true ) => encoded.extend_from_slice(&[0x88, id]),
                (true,  true ) => encoded.push(0xc0 + id),
            };
            components.push(ComponentIdx {
                id: id as u16,
                idx: idx as u32,
            });
        }

        assert_eq!(check_entity_data_round_trip(&encoded).components, components);
    }

    // error: bad u16 component idx count
    let mut encoded = Vec::new();
    encoded.push(0xff);
    for _ in 0..0xff {
        encoded.push(0xc0);
    }
    assert!(decode_entity_data(&encoded).is_err());
    
    // ok: correct u16 component idx count
    encoded.insert(1, 0xff);
    encoded.insert(1, 0x00);
    check_entity_data_round_trip(&encoded);
}
