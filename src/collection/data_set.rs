#![allow(unused)]

use paste::paste;

#[macro_export]
macro_rules! data_set_properties_counter {
    (
        properties: [$property:ident -> $property_type:ty, $value:expr]
    ) =>
    {
        $value
    };

    (
        properties: [
        $property:ident -> $property_type:ty,
        $previous_value:expr,
        $($rest_property:ident -> $rest_property_type:ty, $rest_value:expr),* $(,)?
        ]
    ) =>
    {
        data_set_properties_counter!{
            properties: [$($rest_property -> $rest_property_type, $previous_value + 1usize),*]
        }
    };
}
#[macro_export]
macro_rules! impl_data_set_key_trait {
    (
        name: $name:ident,
        enum_name: $enum_name:ident,
        key: $property:ident,
        data: $property_type:ty,
        idx: $value:expr
    ) => {
        ::paste::paste! {
            impl [<TKeyAliasConstraint $name>]<$name> for $property
            {
                type Data = $property_type;

                fn index() -> usize { $value }

                fn extract(val: &$enum_name) -> Option<&Self::Data>
                {
                    if let $enum_name::$property(v) = val
                    {
                        Some(v)
                    }
                    else
                    {
                        None
                    }
                }

                fn extract_mut(val: &mut $enum_name) -> Option<&mut Self::Data>
                {
                    if let $enum_name::$property(v) = val
                    {
                        Some(v)
                    }
                    else
                    {
                        None
                    }
                }
            }
        }
    };
}
#[macro_export]
macro_rules! data_set_key_trait_incrementor {
    // when collection contains only ONE element
    (
        name: $name:ident,
        enum_name: $enum_name:ident,
        properties: [$property:ident -> $property_type:ty, $value:expr]
        ) => {

        impl_data_set_key_trait!{
            name: $name,
            enum_name: $enum_name,
            key: $property,
            data: $property_type,
            idx: $value
        }
    };
    // when collection contains more than ONE element
    (
        name: $name:ident,
        enum_name: $enum_name:ident,
        properties: [
        $property:ident -> $property_type:ty,
        $previous_value:expr,
        $($rest_property:ident -> $rest_property_type:ty, $rest_value:expr),* $(,)?
        ]
        ) =>
    {
        impl_data_set_key_trait!{
            name: $name,
            enum_name: $enum_name,
            key: $property,
            data: $property_type,
            idx: $previous_value
        }

        data_set_key_trait_incrementor!
       {
            name: $name,
            enum_name: $enum_name,
            properties: [$($rest_property -> $rest_property_type, $previous_value + 1usize),*]
        }
    };
}
/// # `xynok_std::collection::data_set!`
/// - [issue](https://github.com/monok-robeto/xynok-engine/issues/10)
/// - [reference](https://stackoverflow.com/questions/79294124/increment-constants-inside-macro)
#[macro_export]
macro_rules! data_set {
    (
        name: $name:ident,
        enum_name: $enum_name:ident,
        properties: [$($property:ident -> $property_type:ty),+ $(,)?]
    ) => {
        ::paste::paste!
        {

            use $crate::{data_set_properties_counter, impl_data_set_key_trait, data_set_key_trait_incrementor};
            pub enum $enum_name
            {
                $($property($property_type),)+
            }
            // struct type keys
            $(pub struct $property;)+


                pub trait [<TKeyAliasConstraint $name>]<AliasDataSet>
                {
                    type Data;
                    fn index() -> usize;
                    fn extract(val: &$enum_name) -> Option<&Self::Data>;
                    fn extract_mut(val: &mut $enum_name) -> Option<&mut Self::Data>;
                }


            data_set_key_trait_incrementor!
            {
                name: $name,
                enum_name: $enum_name,
                properties: [$($property -> $property_type, 0usize),*]
            }
            pub struct $name
            {
                slots: Vec<Option<$enum_name>>
            }
            impl Default for $name
            {
                fn default() -> Self
                {
                    Self::new()
                }
            }
            impl $name
            {
                pub const TOTAL_PROPERTIES: usize = data_set_properties_counter!
                {
                    properties: [$($property -> $property_type, 1usize),*]
                };

                pub fn new() -> Self
                {
                    Self
                    {
                        slots: (0..Self::TOTAL_PROPERTIES).map(|_| None).collect()
                    }
                }

                pub fn exists<K: [<TKeyAliasConstraint $name>]<Self>>(&self, _key: K) -> bool
                {
                    let idx = K::index();
                    self.slots[idx].is_some()
                }

                pub fn add<K: [<TKeyAliasConstraint $name>]<Self>>(&mut self, _key: K, val: $enum_name) -> Result<(), &'static str>
                {
                    let idx = K::index();
                    if self.slots[idx].is_some()
                    {
                        Err("slot already occupied - use get_mut() to edit, or remove() first !")
                    }
                    else
                    {
                        self.slots[idx] = Some(val);
                        Ok(())
                    }
                }

                pub fn remove<K: [<TKeyAliasConstraint $name>]<Self>>(&mut self, _key: K) -> Option<$enum_name>
                {
                    let idx = K::index();
                    self.slots[idx].take()
                }

                pub fn get<K:[<TKeyAliasConstraint $name>]<Self>>(&self, _key: K) -> Option<&K::Data>
                {
                    let idx = K::index();
                    let reference = self.slots[idx].as_ref()?;
                    K::extract(reference)
                }
                pub fn get_mut<K: [<TKeyAliasConstraint $name>]<Self>>(&mut self, _key: K) -> Option<&mut K::Data>
                {
                    let idx = K::index();
                    let reference = self.slots[idx].as_mut()?;
                    K::extract_mut(reference)
                }
            }
        }
    };
}
#[cfg(test)]
mod test
{

    use super::*;
    data_set! {
        name: Person,
        enum_name: PersonData,
        properties: [
            Age -> u32,
            Height -> f32,
            Weight -> f32,
        ]
    }

    #[test]
    fn exists()
    {
        let mut person = Person::new();
        assert!(!person.exists(Age));
    }

    #[test]
    fn add_check_exists()
    {
        let mut person = Person::new();
        assert!(!person.exists(Age));
        person.add(Age, PersonData::Age(12u32));
        assert!(person.exists(Age));
    }
    #[test]
    fn add_multiple_key_check_exists()
    {
        let mut person = Person::new();
        assert!(!person.exists(Age));
        person.add(Age, PersonData::Age(12u32));
        person.add(Height, PersonData::Height(1f32));
        assert!(person.exists(Age));
        assert!(person.exists(Height));
    }
    #[test]
    fn add_remove_and_check_exists()
    {
        let mut person = Person::new();
        person.add(Age, PersonData::Age(12u32));
        person.add(Height, PersonData::Height(1f32));

        assert!(person.exists(Age));

        person.remove(Age);
        assert!(!person.exists(Age));

        person.remove(Height);
        assert!(!person.exists(Height));
    }

    #[test]
    fn add_check_exist_and_get()
    {
        let mut person = Person::new();
        person.add(Age, PersonData::Age(12u32));

        assert!(person.exists(Age));
        if let Some(age) = person.get(Age)
        {
            assert!(*age == 12u32);
        }
    }
    #[test]
    fn add_check_exist_and_get_mut()
    {
        let mut person = Person::new();
        person.add(Age, PersonData::Age(12u32));

        assert!(person.exists(Age));
        let age = person.get_mut(Age).unwrap();
        assert!(*age == 12u32);
        *age = 122u32;

        if let Some(age) = person.get(Age)
        {
            assert!(*age == 122u32);
        }
    }
}
