use std::{fs, io, marker::PhantomData, path::Path};

pub trait Serializable: Sized {
    fn serialize(&self, serializer: &mut Serializer);
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()>;
}

#[derive(PartialEq, Eq, Debug)]
pub enum PrimitiveType {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
}

#[derive(PartialEq, Eq, Debug)]
pub enum ValueType {
    Primitive(PrimitiveType),

    // A list of values of a common type whose
    // number of elements can be queried
    Array(PrimitiveType),

    // A utf-8 encoded string
    String,

    // A logically grouped sequence of values
    // whose size in bytes can be queried,
    // useful during serialization to safely
    // divide work into non-overlapping chunks
    SubArchive,
}

impl PrimitiveType {
    fn to_nibble(&self) -> u8 {
        match self {
            PrimitiveType::Bool => 0x01,
            PrimitiveType::U8 => 0x02,
            PrimitiveType::I8 => 0x03,
            PrimitiveType::U16 => 0x04,
            PrimitiveType::I16 => 0x05,
            PrimitiveType::U32 => 0x06,
            PrimitiveType::I32 => 0x07,
            PrimitiveType::U64 => 0x08,
            PrimitiveType::I64 => 0x09,
            PrimitiveType::F32 => 0x0A,
            PrimitiveType::F64 => 0x0B,
        }
    }

    fn from_nibble(byte: u8) -> Result<PrimitiveType, ()> {
        match byte {
            0x01 => Ok(PrimitiveType::Bool),
            0x02 => Ok(PrimitiveType::U8),
            0x03 => Ok(PrimitiveType::I8),
            0x04 => Ok(PrimitiveType::U16),
            0x05 => Ok(PrimitiveType::I16),
            0x06 => Ok(PrimitiveType::U32),
            0x07 => Ok(PrimitiveType::I32),
            0x08 => Ok(PrimitiveType::U64),
            0x09 => Ok(PrimitiveType::I64),
            0x0A => Ok(PrimitiveType::F32),
            0x0B => Ok(PrimitiveType::F64),
            _ => Err(()),
        }
    }
}

impl ValueType {
    fn to_byte(&self) -> u8 {
        match self {
            ValueType::Primitive(prim_type) => (0x00 | prim_type.to_nibble()),
            ValueType::Array(prim_type) => (0x10 | prim_type.to_nibble()),
            ValueType::String => 0x20,
            ValueType::SubArchive => 0x30,
        }
    }

    fn from_byte(byte: u8) -> Result<ValueType, ()> {
        let hi_nibble = byte & 0xF0;
        let lo_nibble = byte & 0x0F;
        match hi_nibble {
            0x00 => Ok(ValueType::Primitive(PrimitiveType::from_nibble(lo_nibble)?)),
            0x10 => Ok(ValueType::Array(PrimitiveType::from_nibble(lo_nibble)?)),
            0x20 => Ok(ValueType::String),
            0x30 => Ok(ValueType::SubArchive),
            _ => Err(()),
        }
    }
}

trait PrimitiveReadWrite: Sized {
    const SIZE: usize;
    const TYPE: PrimitiveType;
    fn write_to(&self, data: &mut Vec<u8>);

    // Precondition: the deserializer has a remaining length of
    // at least `Self::SIZE` bytes
    fn read_from(data: &mut Deserializer) -> Self;
}

macro_rules! impl_primitive_read_write {
    ($primitive: ident, $size: literal, $typetag: expr) => {
        impl PrimitiveReadWrite for $primitive {
            const SIZE: usize = $size;
            const TYPE: PrimitiveType = $typetag;
            fn write_to(&self, data: &mut Vec<u8>) {
                for b in self.to_be_bytes() {
                    data.push(b);
                }
            }
            fn read_from(d: &mut Deserializer) -> Self {
                let mut bytes = Self::default().to_be_bytes();
                for b in &mut bytes {
                    *b = d.read_byte().unwrap();
                }
                Self::from_be_bytes(bytes)
            }
        }
    };
}

impl_primitive_read_write!(u8, 1, PrimitiveType::U8);
impl_primitive_read_write!(i8, 1, PrimitiveType::I8);
impl_primitive_read_write!(u16, 2, PrimitiveType::U16);
impl_primitive_read_write!(i16, 2, PrimitiveType::I16);
impl_primitive_read_write!(u32, 4, PrimitiveType::U32);
impl_primitive_read_write!(i32, 4, PrimitiveType::I32);
impl_primitive_read_write!(u64, 8, PrimitiveType::U64);
impl_primitive_read_write!(i64, 8, PrimitiveType::I64);
impl_primitive_read_write!(f32, 4, PrimitiveType::F32);
impl_primitive_read_write!(f64, 8, PrimitiveType::F64);

impl PrimitiveReadWrite for bool {
    const SIZE: usize = 1;
    const TYPE: PrimitiveType = PrimitiveType::Bool;

    fn write_to(&self, data: &mut Vec<u8>) {
        data.push(if *self { 1 } else { 0 });
    }

    fn read_from(d: &mut Deserializer) -> bool {
        d.read_byte().unwrap() != 0
    }
}

pub struct Archive {
    data: Vec<u8>,
}

impl Archive {
    pub fn serialize_with<F: Fn(Serializer)>(f: F) -> Archive {
        let mut data = Vec::<u8>::new();
        let serializer = Serializer::new_with_prefix_len(&mut data);
        f(serializer);
        Archive { data }
    }

    pub fn deserialize<'a>(&'a self) -> Result<Deserializer<'a>, ()> {
        if self.data.len() < 4 {
            return Err(());
        }
        let len =
            u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]) as usize;
        let slice = &self.data[4..];
        if len != slice.len() {
            return Err(());
        }
        Ok(Deserializer {
            data: slice,
            position: 0,
        })
    }

    pub fn dump_to_file(&self, path: &Path) -> Result<(), io::Error> {
        fs::write(path, &self.data)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Archive, io::Error> {
        let data = fs::read(path)?;
        Ok(Archive { data })
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    pub fn from_vec(data: Vec<u8>) -> Archive {
        Archive { data }
    }
}

pub struct Serializer<'a> {
    data: &'a mut Vec<u8>,
    start_index: usize,
}

impl<'a> Serializer<'a> {
    fn new_with_prefix_len(data: &'a mut Vec<u8>) -> Serializer<'a> {
        let start_index = data.len();
        let placeholder_len: u32 = 0;
        placeholder_len.write_to(data);
        Serializer { data, start_index }
    }

    fn write_primitive<T: PrimitiveReadWrite>(&mut self, x: T) {
        self.data.reserve(u8::SIZE + T::SIZE);
        self.data.push(ValueType::Primitive(T::TYPE).to_byte());
        x.write_to(self.data);
    }

    fn write_primitive_array_slice<T: PrimitiveReadWrite>(&mut self, x: &[T]) {
        self.data
            .reserve(u8::SIZE + u32::SIZE + (x.len() * T::SIZE));
        self.data.push(ValueType::Array(T::TYPE).to_byte());
        let len = x.len() as u32;
        len.write_to(self.data);
        for xi in x {
            xi.write_to(self.data);
        }
    }

    fn write_primitive_array_iter<I: Iterator>(&mut self, it: I)
    where
        I::Item: PrimitiveReadWrite,
    {
        self.data.push(ValueType::Array(I::Item::TYPE).to_byte());
        let array_start_index = self.data.len();
        let mut n_items: u32 = 0;
        n_items.write_to(self.data);
        while let Some(x) = it.next() {
            x.write_to(self.data);
            n_items += 1;
        }
        for (i, b) in n_items.to_be_bytes().iter().enumerate() {
            self.data[array_start_index + i] = *b;
        }
    }

    pub fn u8(&mut self, x: u8) {
        self.write_primitive::<u8>(x);
    }
    pub fn i8(&mut self, x: i8) {
        self.write_primitive::<i8>(x);
    }
    pub fn u16(&mut self, x: u16) {
        self.write_primitive::<u16>(x);
    }
    pub fn i16(&mut self, x: i16) {
        self.write_primitive::<i16>(x);
    }
    pub fn u32(&mut self, x: u32) {
        self.write_primitive::<u32>(x);
    }
    pub fn i32(&mut self, x: i32) {
        self.write_primitive::<i32>(x);
    }
    pub fn u64(&mut self, x: u64) {
        self.write_primitive::<u64>(x);
    }
    pub fn i64(&mut self, x: i64) {
        self.write_primitive::<i64>(x);
    }
    pub fn f32(&mut self, x: f32) {
        self.write_primitive::<f32>(x);
    }
    pub fn f64(&mut self, x: f64) {
        self.write_primitive::<f64>(x);
    }

    pub fn array_slice_u8(&mut self, x: &[u8]) {
        self.write_primitive_array_slice::<u8>(x);
    }
    pub fn array_slice_i8(&mut self, x: &[i8]) {
        self.write_primitive_array_slice::<i8>(x);
    }
    pub fn array_slice_u16(&mut self, x: &[u16]) {
        self.write_primitive_array_slice::<u16>(x);
    }
    pub fn array_slice_i16(&mut self, x: &[i16]) {
        self.write_primitive_array_slice::<i16>(x);
    }
    pub fn array_slice_u32(&mut self, x: &[u32]) {
        self.write_primitive_array_slice::<u32>(x);
    }
    pub fn array_slice_i32(&mut self, x: &[i32]) {
        self.write_primitive_array_slice::<i32>(x);
    }
    pub fn array_slice_u64(&mut self, x: &[u64]) {
        self.write_primitive_array_slice::<u64>(x);
    }
    pub fn array_slice_i64(&mut self, x: &[i64]) {
        self.write_primitive_array_slice::<i64>(x);
    }
    pub fn array_slice_f32(&mut self, x: &[f32]) {
        self.write_primitive_array_slice::<f32>(x);
    }
    pub fn array_slice_f64(&mut self, x: &[f64]) {
        self.write_primitive_array_slice::<f64>(x);
    }

    pub fn array_iter_u8<I: Iterator<Item = u8>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_i8<I: Iterator<Item = i8>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_u16<I: Iterator<Item = u16>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_i16<I: Iterator<Item = i16>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_u32<I: Iterator<Item = u32>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_i32<I: Iterator<Item = i32>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_u64<I: Iterator<Item = u64>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_i64<I: Iterator<Item = i64>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_f32<I: Iterator<Item = f32>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }
    pub fn array_iter_f64<I: Iterator<Item = f64>>(&mut self, it: I) {
        self.write_primitive_array_iter(it);
    }

    pub fn string(&mut self, x: &str) {
        let bytes = x.as_bytes();
        self.data.reserve(u8::SIZE + u32::SIZE + bytes.len());
        self.data.push(ValueType::String.to_byte());
        let len = bytes.len() as u32;
        len.write_to(self.data);
        for b in bytes {
            self.data.push(*b);
        }
    }

    pub fn subarchive<'b>(&'b mut self) -> Serializer<'b> {
        self.data.push(ValueType::SubArchive.to_byte());
        Serializer::new_with_prefix_len(self.data)
    }

    pub fn object<T: Serializable>(&mut self, object: &T) {
        object.serialize(self);
    }
}

impl<'a> Drop for Serializer<'a> {
    fn drop(&mut self) {
        let new_len = self.data.len();
        let delta_len = new_len - self.start_index;
        debug_assert!(delta_len >= u32::SIZE);
        let subarchive_line = (delta_len - u32::SIZE) as u32;
        for (i, b) in subarchive_line.to_be_bytes().iter().enumerate() {
            self.data[self.start_index + i] = *b;
        }
    }
}

pub struct DeserializerIterator<'a, T> {
    deserializer: &'a mut Deserializer<'a>,
    remaining: usize,
    _phantom_data: PhantomData<T>,
}

impl<'a, T> DeserializerIterator<'a, T> {
    fn new(
        deserializer: &'a mut Deserializer<'a>,
        remaining: usize,
    ) -> DeserializerIterator<'a, T> {
        DeserializerIterator {
            deserializer,
            remaining,
            _phantom_data: PhantomData,
        }
    }
}

impl<'a, T: PrimitiveReadWrite> Iterator for DeserializerIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(T::read_from(self.deserializer))
    }
}

pub struct Deserializer<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Deserializer<'a> {
    fn new(data: &'a [u8]) -> Deserializer<'a> {
        Deserializer { data, position: 0 }
    }

    fn remaining_len(&self) -> usize {
        let l = self.data.len();
        debug_assert!(self.position <= l);
        return l - self.position;
    }

    fn read_byte(&mut self) -> Result<u8, ()> {
        if self.position >= self.data.len() {
            Err(())
        } else {
            let b = self.data[self.position];
            self.position += 1;
            Ok(b)
        }
    }

    fn peek_byte(&self, offset: usize) -> Result<u8, ()> {
        if (self.position + offset) >= self.data.len() {
            Err(())
        } else {
            Ok(self.data[self.position + offset])
        }
    }

    fn read_primitive<T: PrimitiveReadWrite>(&mut self) -> Result<T, ()> {
        if self.remaining_len() < (u8::SIZE + T::SIZE) {
            return Err(());
        }
        let the_type = ValueType::from_byte(self.read_byte()?)?;
        if the_type != ValueType::Primitive(T::TYPE) {
            return Err(());
        }
        Ok(T::read_from(self))
    }

    fn read_primitive_array_slice<T: PrimitiveReadWrite>(&mut self) -> Result<Vec<T>, ()> {
        if self.remaining_len() < (u8::SIZE + u32::SIZE) {
            return Err(());
        }
        let the_type = ValueType::from_byte(self.read_byte()?)?;
        if the_type != ValueType::Array(T::TYPE) {
            return Err(());
        }
        let len = u32::read_from(self) as usize;
        if self.remaining_len() < (len * T::SIZE) {
            return Err(());
        }
        Ok((0..len).map(|_| T::read_from(self)).collect())
    }

    fn read_primitive_array_iter<T: PrimitiveReadWrite>(
        &'a mut self,
    ) -> Result<DeserializerIterator<'a, T>, ()> {
        if self.remaining_len() < (u8::SIZE + u32::SIZE) {
            return Err(());
        }
        let the_type = ValueType::from_byte(self.read_byte()?)?;
        if the_type != ValueType::Array(T::TYPE) {
            return Err(());
        }
        let len = u32::read_from(self) as usize;
        if self.remaining_len() < (len * T::SIZE) {
            return Err(());
        }
        Ok(DeserializerIterator::new(self, len))
    }

    pub fn u8(&mut self) -> Result<u8, ()> {
        self.read_primitive::<u8>()
    }
    pub fn i8(&mut self) -> Result<i8, ()> {
        self.read_primitive::<i8>()
    }
    pub fn u16(&mut self) -> Result<u16, ()> {
        self.read_primitive::<u16>()
    }
    pub fn i16(&mut self) -> Result<i16, ()> {
        self.read_primitive::<i16>()
    }
    pub fn u32(&mut self) -> Result<u32, ()> {
        self.read_primitive::<u32>()
    }
    pub fn i32(&mut self) -> Result<i32, ()> {
        self.read_primitive::<i32>()
    }
    pub fn u64(&mut self) -> Result<u64, ()> {
        self.read_primitive::<u64>()
    }
    pub fn i64(&mut self) -> Result<i64, ()> {
        self.read_primitive::<i64>()
    }
    pub fn f32(&mut self) -> Result<f32, ()> {
        self.read_primitive::<f32>()
    }
    pub fn f64(&mut self) -> Result<f64, ()> {
        self.read_primitive::<f64>()
    }

    pub fn array_slice_u8(&mut self) -> Result<Vec<u8>, ()> {
        self.read_primitive_array_slice::<u8>()
    }
    pub fn array_slice_i8(&mut self) -> Result<Vec<i8>, ()> {
        self.read_primitive_array_slice::<i8>()
    }
    pub fn array_slice_u16(&mut self) -> Result<Vec<u16>, ()> {
        self.read_primitive_array_slice::<u16>()
    }
    pub fn array_slice_i16(&mut self) -> Result<Vec<i16>, ()> {
        self.read_primitive_array_slice::<i16>()
    }
    pub fn array_slice_u32(&mut self) -> Result<Vec<u32>, ()> {
        self.read_primitive_array_slice::<u32>()
    }
    pub fn array_slice_i32(&mut self) -> Result<Vec<i32>, ()> {
        self.read_primitive_array_slice::<i32>()
    }
    pub fn array_slice_u64(&mut self) -> Result<Vec<u64>, ()> {
        self.read_primitive_array_slice::<u64>()
    }
    pub fn array_slice_i64(&mut self) -> Result<Vec<i64>, ()> {
        self.read_primitive_array_slice::<i64>()
    }
    pub fn array_slice_f32(&mut self) -> Result<Vec<f32>, ()> {
        self.read_primitive_array_slice::<f32>()
    }
    pub fn array_slice_f64(&mut self) -> Result<Vec<f64>, ()> {
        self.read_primitive_array_slice::<f64>()
    }

    pub fn array_iter_f32(&'a mut self) -> Result<DeserializerIterator<'a, f32>, ()> {
        self.read_primitive_array_iter::<f32>()
    }

    pub fn string(&mut self) -> Result<String, ()> {
        if self.remaining_len() < (u8::SIZE + u32::SIZE) {
            return Err(());
        }
        let the_type = ValueType::from_byte(self.read_byte()?)?;
        if the_type != ValueType::String {
            return Err(());
        }
        let len = u32::read_from(self) as usize;
        if self.remaining_len() < len {
            return Err(());
        }
        let slice = &self.data[self.position..(self.position + len)];
        self.position += len;
        let str_slice = std::str::from_utf8(slice).map_err(|_| ())?;
        Ok(str_slice.to_string())
    }

    pub fn subarchive<'b>(&'b mut self) -> Result<Deserializer<'b>, ()> {
        if self.remaining_len() < (u8::SIZE + u32::SIZE) {
            return Err(());
        }
        let the_type = ValueType::from_byte(self.read_byte()?)?;
        if the_type != ValueType::SubArchive {
            return Err(());
        }
        let len = u32::read_from(self) as usize;
        if self.remaining_len() < len {
            return Err(());
        }
        let subarchive_slice: &[u8] = &self.data[self.position..(self.position + len)];
        self.position += len;
        Ok(Deserializer::new(subarchive_slice))
    }

    pub fn object<T: Serializable>(&mut self) -> Result<T, ()> {
        T::deserialize(self)
    }

    pub fn peek_type(&self) -> Result<ValueType, ()> {
        ValueType::from_byte(self.peek_byte(0)?)
    }

    pub fn peek_length(&self) -> Result<usize, ()> {
        let the_type = ValueType::from_byte(self.peek_byte(0)?)?;
        if let ValueType::Primitive(_) = the_type {
            return Err(());
        }
        Ok(u32::from_be_bytes([
            self.peek_byte(1)?,
            self.peek_byte(2)?,
            self.peek_byte(3)?,
            self.peek_byte(4)?,
        ]) as usize)
    }

    pub fn is_empty(&self) -> bool {
        return self.position == self.data.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
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
            assert_eq!(d2.remaining_len(), 0);
        }

        assert_eq!(
            d1.peek_type().unwrap(),
            ValueType::Primitive(PrimitiveType::U8)
        );
        assert!(d1.peek_length().is_err());
        assert_eq!(d1.u8().unwrap(), 1);

        assert!(d1.peek_type().is_err());
        assert!(d1.peek_length().is_err());
        assert_eq!(d1.remaining_len(), 0);
    }
}
