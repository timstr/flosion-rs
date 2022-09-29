use crate::core::serialization::{Archive, PrimitiveType, ValueType};

#[test]
fn end_to_end_test() {
    let archive = Archive::serialize_with(|mut s1| {
        s1.u8(1);
        s1.u16(2);
        s1.array_slice_u8(&[0, 1, 2, 3]);
        s1.array_slice_u64(&[10, 11, 12, 13]);
        s1.string("Testing");

        {
            let mut s2 = s1.subarchive();

            // YASSSS this should fail
            // s1.u8(0);

            s2.u8(3);
            s2.u16(4);
            s2.array_slice_u8(&[20, 21, 22, 23]);
            s2.array_slice_u64(&[30, 31, 32, 33]);
        }
        s1.u8(1);
    });

    let mut d1 = archive.deserialize().unwrap();
    assert_eq!(
        d1.peek_type().unwrap(),
        ValueType::Primitive(PrimitiveType::U8)
    );
    assert!(d1.peek_length().is_err());
    assert_eq!(d1.u8().unwrap(), 1);
    assert_eq!(
        d1.peek_type().unwrap(),
        ValueType::Primitive(PrimitiveType::U16)
    );
    assert!(d1.peek_length().is_err());
    assert_eq!(d1.u16().unwrap(), 2);
    assert_eq!(d1.peek_type().unwrap(), ValueType::Array(PrimitiveType::U8));
    assert_eq!(d1.peek_length().unwrap(), 4);
    assert_eq!(d1.array_slice_u8().unwrap(), vec![0, 1, 2, 3]);
    assert_eq!(
        d1.peek_type().unwrap(),
        ValueType::Array(PrimitiveType::U64)
    );
    assert_eq!(d1.peek_length().unwrap(), 4);
    assert_eq!(d1.array_slice_u64().unwrap(), vec![10, 11, 12, 13]);
    assert_eq!(d1.peek_type().unwrap(), ValueType::String);
    assert_eq!(d1.peek_length().unwrap(), "Testing".as_bytes().len());
    assert_eq!(d1.string().unwrap(), "Testing");

    assert_eq!(d1.peek_type().unwrap(), ValueType::SubArchive);
    {
        let mut d2 = d1.subarchive().unwrap();
        assert_eq!(
            d2.peek_type().unwrap(),
            ValueType::Primitive(PrimitiveType::U8)
        );
        assert!(d2.peek_length().is_err());
        assert_eq!(d2.u8().unwrap(), 3);
        assert_eq!(
            d2.peek_type().unwrap(),
            ValueType::Primitive(PrimitiveType::U16)
        );
        assert!(d2.peek_length().is_err());
        assert_eq!(d2.u16().unwrap(), 4);
        assert_eq!(d2.peek_type().unwrap(), ValueType::Array(PrimitiveType::U8));
        assert_eq!(d2.peek_length().unwrap(), 4);
        assert_eq!(d2.array_slice_u8().unwrap(), vec![20, 21, 22, 23]);
        assert_eq!(
            d2.peek_type().unwrap(),
            ValueType::Array(PrimitiveType::U64)
        );
        assert_eq!(d2.peek_length().unwrap(), 4);
        assert_eq!(d2.array_slice_u64().unwrap(), vec![30, 31, 32, 33]);
        assert!(d2.peek_type().is_err());
        assert!(d2.peek_length().is_err());
        assert!(d2.is_empty());
    }

    assert_eq!(
        d1.peek_type().unwrap(),
        ValueType::Primitive(PrimitiveType::U8)
    );
    assert!(d1.peek_length().is_err());
    assert_eq!(d1.u8().unwrap(), 1);

    assert!(d1.peek_type().is_err());
    assert!(d1.peek_length().is_err());
    assert!(d1.is_empty());
}

#[test]
fn empty_subarchive_test() {
    let archive = Archive::serialize_with(|mut s1| {
        s1.u8(0xAA);
        {
            s1.subarchive();
        }
        s1.u8(0xBB);
    });

    let mut d1 = archive.deserialize().unwrap();
    assert_eq!(d1.u8().unwrap(), 0xAA);
    {
        let d2 = d1.subarchive().unwrap();
        assert!(d2.is_empty());
    }
    assert_eq!(d1.u8().unwrap(), 0xBB);
}

#[test]
fn nested_empty_subarchive_test() {
    let archive = Archive::serialize_with(|mut s1| {
        s1.u8(0xAA);
        {
            let mut s2 = s1.subarchive();
            s2.u8(0x11);
            {
                s2.subarchive();
            }
            s2.u8(0x22);
        }
        s1.u8(0xBB);
    });

    let mut d1 = archive.deserialize().unwrap();
    assert_eq!(d1.u8().unwrap(), 0xAA);
    {
        let mut d2 = d1.subarchive().unwrap();
        assert_eq!(d2.u8().unwrap(), 0x11);
        {
            let d3 = d2.subarchive().unwrap();
            assert!(d3.is_empty());
        }
        assert_eq!(d2.u8().unwrap(), 0x22);
        assert!(d2.is_empty());
    }
    assert_eq!(d1.u8().unwrap(), 0xBB);
    assert!(d1.is_empty());
}
