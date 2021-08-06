use super::SerdeError;
use crate::{repr::Repr, yaml_map, Array, Map, NodeBase};
use core::{fmt::Display, marker::PhantomData};
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    serde_if_integer128, Serialize, Serializer,
};

macro_rules! impl_serializer {
    (@) => { () };
    (@$ty:ty, $name:ident) => { $name };
    ($(fn $method:ident$(($ty:ty))?)+) => {
        $(fn $method(self$(, v: $ty)?) -> Result<Self::Ok, Self::Error> {
            Ok(impl_serializer!(@$($ty, v)?).into())
        })+
    };
}

pub fn to_node(any: impl Serialize) -> Result<crate::Node, SerdeError> {
    any.serialize(NodeSerializer(PhantomData))
}

pub fn to_arc_node(any: impl Serialize) -> Result<crate::ArcNode, SerdeError> {
    any.serialize(NodeSerializer(PhantomData))
}

struct NodeSerializer<R: Repr>(PhantomData<R>);

impl<R: Repr> Serializer for NodeSerializer<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;
    type SerializeSeq = SeqSerializer<R>;
    type SerializeTuple = SeqSerializer<R>;
    type SerializeTupleStruct = SeqSerializer<R>;
    type SerializeTupleVariant = TupleVariant<R>;
    type SerializeMap = MapSerializer<R>;
    type SerializeStruct = StructSerializer<R>;
    type SerializeStructVariant = StructVariant<R>;

    impl_serializer! {
        fn serialize_bool(bool)
        fn serialize_i8(i8)
        fn serialize_i16(i16)
        fn serialize_i32(i32)
        fn serialize_i64(i64)
        fn serialize_u8(u8)
        fn serialize_u16(u16)
        fn serialize_u32(u32)
        fn serialize_u64(u64)
        fn serialize_f32(f32)
        fn serialize_f64(f64)
        fn serialize_char(char)
        fn serialize_str(&str)
        fn serialize_none
        fn serialize_unit
    }

    serde_if_integer128! {
        impl_serializer! {
            fn serialize_i128(i128)
            fn serialize_u128(u128)
        }
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(v.iter().map(|b| NodeBase::from(*b)).collect())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(().into())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant.into())
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Ok(yaml_map!(variant.into() => value.serialize(NodeSerializer(PhantomData))?).into())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let array = match len {
            Some(n) => Array::with_capacity(n),
            None => Array::new(),
        };
        Ok(SeqSerializer(array))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariant(Array::with_capacity(len), variant))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer(
            match len {
                Some(n) => Map::with_capacity(n),
                None => Map::new(),
            },
            None,
        ))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer(Map::with_capacity(len)))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariant(Map::with_capacity(len), variant))
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Display,
    {
        use alloc::string::ToString;
        self.serialize_str(&value.to_string())
    }
}

struct SeqSerializer<R: Repr>(Array<R>);

impl<R: Repr> SerializeSeq for SeqSerializer<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.push(value.serialize(NodeSerializer(PhantomData))?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.into())
    }
}

impl<R: Repr> SerializeTuple for SeqSerializer<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<R: Repr> SerializeTupleStruct for SeqSerializer<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

struct TupleVariant<R: Repr>(Array<R>, &'static str);

impl<R: Repr> SerializeTupleVariant for TupleVariant<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.push(value.serialize(NodeSerializer(PhantomData))?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(yaml_map!(self.1.into() => self.0.into()).into())
    }
}

struct MapSerializer<R: Repr>(Map<R>, Option<NodeBase<R>>);

impl<R: Repr> SerializeMap for MapSerializer<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.1 = Some(key.serialize(NodeSerializer(PhantomData))?);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        match self.1.take() {
            Some(k) => self
                .0
                .insert(k, value.serialize(NodeSerializer(PhantomData))?),
            None => panic!("serialize_value called before serialize_key"),
        };
        Ok(())
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        self.0.insert(
            key.serialize(NodeSerializer(PhantomData))?,
            value.serialize(NodeSerializer(PhantomData))?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.into())
    }
}

struct StructSerializer<R: Repr>(Map<R>);

impl<R: Repr> SerializeStruct for StructSerializer<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.insert(
            key.serialize(NodeSerializer(PhantomData))?,
            value.serialize(NodeSerializer(PhantomData))?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.0.into())
    }
}

struct StructVariant<R: Repr>(Map<R>, &'static str);

impl<R: Repr> SerializeStructVariant for StructVariant<R> {
    type Ok = NodeBase<R>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.insert(
            key.serialize(NodeSerializer(PhantomData))?,
            value.serialize(NodeSerializer(PhantomData))?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(yaml_map!(self.1.into() => self.0.into()).into())
    }
}