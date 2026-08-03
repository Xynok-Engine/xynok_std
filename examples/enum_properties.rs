/// define_container! — zero-overhead typed container via struct keys + associated types.
///
/// No Any, no downcast, no runtime casting — pure static dispatch.
///
/// Syntax:
///   define_container! {
///       ContainerName,
///       ValueEnum,
///       [
///           KeyStruct => Type,
///           ...
///       ]
///   }
macro_rules! define_container {
    (
        $container:ident,
        $value_enum:ident,
        [ $( $key:ident => $type:ty ),+ $(,)? ]
    ) => {

        // ----------------------------------------------------------------
        // One ZST key struct per variant
        // ----------------------------------------------------------------
        $( pub struct $key; )+

        // ----------------------------------------------------------------
        // Value enum — carries actual data, variant name == key struct name
        // ----------------------------------------------------------------
        #[derive(Debug, Clone)]
        pub enum $value_enum {
            $( $key($type), )+
        }

        // ----------------------------------------------------------------
        // DataKey trait — each key struct knows how to extract its own type
        // No casting — pure if-let pattern match on the correct variant
        // ----------------------------------------------------------------
        pub trait DataKey<C> {
            type Output;
            fn index() -> usize;
            fn extract(value: &$value_enum) -> Option<&Self::Output>;
            fn extract_mut(value: &mut $value_enum) -> Option<&mut Self::Output>;
        }

        // Implement DataKey for each key struct, recursively with index counter
        define_container!(@impl_keys $container, $value_enum, 0usize,
            $( $key => $type ),+
        );


        // ----------------------------------------------------------------
        // Container struct
        // ----------------------------------------------------------------
        pub struct $container {
            slots: Vec<Option<$value_enum>>,
        }

        impl $container {
            const COUNT: usize = [ $( stringify!($key), )+ ].len();

            /// Creates container with all slots empty.
            pub fn new() -> Self {
                Self {
                    slots: (0..Self::COUNT).map(|_| None).collect(),
                }
            }

            /// Insert value. Returns Err if slot already occupied.
            pub fn add(&mut self, value: $value_enum) -> Result<(), &'static str> {
                let idx = Self::variant_index(&value);
                if self.slots[idx].is_some() {
                    Err("slot already occupied — use get_mut() to edit, or remove() first")
                } else {
                    self.slots[idx] = Some(value);
                    Ok(())
                }
            }

            /// Immutable typed reference. Static dispatch, zero overhead.
            pub fn get<K: DataKey<Self>>(&self, _key: K) -> Option<&K::Output> {
                K::extract(self.slots[K::index()].as_ref()?)
            }

            /// Mutable typed reference. Static dispatch, zero overhead.
            pub fn get_mut<K: DataKey<Self>>(&mut self, _key: K) -> Option<&mut K::Output> {
                K::extract_mut(self.slots[K::index()].as_mut()?)
            }

            /// Remove and return the value at this key's slot.
            pub fn remove<K: DataKey<Self>>(&mut self, _key: K) -> Option<$value_enum> {
                self.slots[K::index()].take()
            }

            /// Returns true if the slot is filled.
            pub fn has<K: DataKey<Self>>(&self, _key: K) -> bool {
                self.slots[K::index()].is_some()
            }

            // Maps a value variant to its slot index — pure match, no casting
            fn variant_index(value: &$value_enum) -> usize {
                define_container!(@variant_index value, $value_enum, 0usize, $( $key ),+)
            }
        }

        impl Default for $container {
            fn default() -> Self { Self::new() }
        }

        impl std::fmt::Debug for $container {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let names = [ $( stringify!($key), )+ ];
                let mut d = f.debug_map();
                for (i, name) in names.iter().enumerate() {
                    d.entry(name, &self.slots[i]);
                }
                d.finish()
            }
        }
    };

    // ----------------------------------------------------------------
    // @impl_keys — base case (last key)
    // ----------------------------------------------------------------
    (@impl_keys $container:ident, $value_enum:ident, $idx:expr, $key:ident => $type:ty) => {
        impl DataKey<$container> for $key {
            type Output = $type;

            #[inline(always)]
            fn index() -> usize { $idx }

            #[inline(always)]
            fn extract(value: &$value_enum) -> Option<&$type> {
                if let $value_enum::$key(v) = value { Some(v) } else { None }
            }

            #[inline(always)]
            fn extract_mut(value: &mut $value_enum) -> Option<&mut $type> {
                if let $value_enum::$key(v) = value { Some(v) } else { None }
            }
        }
    };

    // @impl_keys — recursive case
    (@impl_keys $container:ident, $value_enum:ident, $idx:expr,
        $key:ident => $type:ty, $( $rest_key:ident => $rest_type:ty ),+
    ) => {
        impl DataKey<$container> for $key {
            type Output = $type;

            #[inline(always)]
            fn index() -> usize { $idx }

            #[inline(always)]
            fn extract(value: &$value_enum) -> Option<&$type> {
                if let $value_enum::$key(v) = value { Some(v) } else { None }
            }

            #[inline(always)]
            fn extract_mut(value: &mut $value_enum) -> Option<&mut $type> {
                if let $value_enum::$key(v) = value { Some(v) } else { None }
            }
        }

        // recurse with index + 1
        define_container!(@impl_keys $container, $value_enum, $idx + 1usize,
            $( $rest_key => $rest_type ),+
        );
    };

    // ----------------------------------------------------------------
    // @variant_index — pure match to map value → slot index, no casting
    // ----------------------------------------------------------------
    (@variant_index $value:ident, $value_enum:ident, $idx:expr, $key:ident) => {
        match $value { $value_enum::$key(_) => $idx, _ => unreachable!() }
    };

    (@variant_index $value:ident, $value_enum:ident, $idx:expr, $key:ident, $( $rest:ident ),+) => {
        match $value {
            $value_enum::$key(_) => $idx,
            _ => define_container!(@variant_index $value, $value_enum, $idx + 1usize, $( $rest ),+),
        }
    };
}

// ============================================================
// Usage — define a PersonData container
// ============================================================

define_container!(
    PersonData,      // container struct name
    PersonValue,     // value enum name (variant name == key struct name)
    [
        Age    => i32,
        Name   => String,
        Score  => f64,
        Active => bool,
    ]
);
pub trait DataKeyS<C>
{
    type Output;
    fn index() -> usize;
    fn extract(value: &PersonValue) -> Option<&Self::Output>;
    fn extract_mut(value: &mut PersonValue) -> Option<&mut Self::Output>;
}
fn main()
{
    let mut person = PersonData::new();

    // All slots start empty
    println!("has Age:  {}", person.has(Age)); // false
    println!("has Name: {}", person.has(Name)); // false

    // add()
    person.add(PersonValue::Age(25)).unwrap();
    person.add(PersonValue::Name("Alice".to_string())).unwrap();
    person.add(PersonValue::Score(9.5)).unwrap();
    person.add(PersonValue::Active(true)).unwrap();

    // double add returns Err — no panic
    println!("{:?}", person.add(PersonValue::Age(99)));
    // Err("slot already occupied...")

    // get() — returns &i32, &String etc. directly — no enum unwrapping needed
    let age: Option<&i32> = person.get(Age);
    let name: Option<&String> = person.get(Name);
    let score: Option<&f64> = person.get(Score);
    let active: Option<&bool> = person.get(Active);

    println!("age={:?} name={:?} score={:?} active={:?}", age, name, score, active);
    // age=Some(25) name=Some("Alice") score=Some(9.5) active=Some(true)

    // get_mut() — &mut i32 directly, edit in place
    if let Some(age) = person.get_mut(Age)
    {
        *age += 1;
    }
    println!("age after mut: {:?}", person.get(Age)); // Some(26)

    if let Some(name) = person.get_mut(Name)
    {
        name.push_str(" Smith");
    }
    println!("name: {:?}", person.get(Name)); // Some("Alice Smith")

    // has()
    println!("has Score: {}", person.has(Score)); // true
    println!("has Active: {}", person.has(Active)); // true

    // remove() — clears slot, returns the value
    let removed = person.remove(Score);
    println!("removed: {:?}", removed); // Some(Score(9.5))
    println!("has Score after remove: {}", person.has(Score)); // false

    // can add back after remove
    person.add(PersonValue::Score(7.0)).unwrap();
    println!("re-added score: {:?}", person.get(Score)); // Some(7.0)

    // Debug print of entire container
    println!("{:#?}", person);
}
