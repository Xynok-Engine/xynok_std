#![allow(unused)]
use std::collections::HashMap;
use std::hash::Hash;
use xynok_std_proc_macro::enum_dispatcher;

// ═════════════════════════════════════════════════════════════════════════════
// CASE 1 — Non-generic trait, non-generic enum
// Simplest case. No generics anywhere.
// ═════════════════════════════════════════════════════════════════════════════

#[enum_dispatcher]
trait KnobControl
{
    fn value(&self) -> f32;
    fn reset(&mut self);
}

struct LinearKnob
{
    value: f32,
}
struct LogarithmicKnob
{
    value: f32,
}

impl KnobControl for LinearKnob
{
    fn value(&self) -> f32 { self.value }
    fn reset(&mut self) { self.value = 0.0; }
}

impl KnobControl for LogarithmicKnob
{
    fn value(&self) -> f32 { self.value.log2() }
    fn reset(&mut self) { self.value = 1.0; }
}

#[enum_dispatcher(KnobControl)]
enum Knob
{
    Linear(LinearKnob),
    Logarithmic(LogarithmicKnob),
}

// Generated:
//   impl KnobControl for Knob {
//       fn value(&self) -> f32 { match self { Knob::Linear(i) => i.value(), ... } }
//       fn reset(&mut self)    { match self { Knob::Linear(i) => i.reset(), ... } }
//   }

fn case1()
{
    let mut knob = Knob::Linear(LinearKnob { value: 5.0 });
    println!("value: {}", knob.value()); // 5.0
    knob.reset();
    println!("after reset: {}", knob.value()); // 0.0
}

// ═════════════════════════════════════════════════════════════════════════════
// CASE 2 — Generic trait, generic enum, generic structs
// Structs must be generic too so the enum can carry T.
// Concrete types only appear at the usage site.
// ═════════════════════════════════════════════════════════════════════════════

#[enum_dispatcher]
trait Processor<T, U>
{
    fn process(&mut self, input: T) -> U;
    fn peek(&self) -> U;
}

// Structs are generic — required because the enum variants hold them with T
struct ScaleProcessor<T>
{
    scale: T,
    last:  T,
}
struct ClampProcessor<T>
{
    min:  T,
    max:  T,
    last: T,
}

impl Processor<f32, f32> for ScaleProcessor<f32>
{
    fn process(&mut self, input: f32) -> f32
    {
        self.last = input * self.scale;
        self.last
    }
    fn peek(&self) -> f32 { self.last }
}

impl Processor<f32, f32> for ClampProcessor<f32>
{
    fn process(&mut self, input: f32) -> f32
    {
        self.last = input.clamp(self.min, self.max);
        self.last
    }
    fn peek(&self) -> f32 { self.last }
}

// T = input type, U = output type (here both are T so enum only needs one param)
#[enum_dispatcher(Processor<T, T>)]
enum AudioEffect<T>
{
    Scale(ScaleProcessor<T>),
    Clamp(ClampProcessor<T>),
}

// Generated:
//   impl<T> Processor<T, T> for AudioEffect<T> {
//       fn process(&mut self, input: T) -> T { match self { ... } }
//       fn peek(&self) -> T                  { match self { ... } }
//   }

fn case2()
{
    let mut effects: Vec<AudioEffect<f32>> = vec![AudioEffect::Scale(ScaleProcessor { scale: 0.5, last: 0.0 }), AudioEffect::Clamp(ClampProcessor { min: -1.0, max: 1.0, last: 0.0 })];

    // 2.0 * 0.5 = 1.0, clamp(1.0, -1.0..1.0) = 1.0
    let result = effects.iter_mut().fold(2.0f32, |s, e| e.process(s));
    println!("pipeline result: {}", result);
}

// ═════════════════════════════════════════════════════════════════════════════
// CASE 3 — Generic trait, generic enum WITH where clause bounds
// ═════════════════════════════════════════════════════════════════════════════

#[enum_dispatcher]
trait Storage<K, V>
{
    fn store(&mut self, key: K, value: V);
    fn load(&self, key: &K) -> Option<&V>;
}

struct VecStorage<K, V>
{
    data: Vec<(K, V)>,
}
struct HashStorage<K, V>
{
    data: HashMap<K, V>,
}

impl<K: PartialEq, V> Storage<K, V> for VecStorage<K, V>
{
    fn store(&mut self, key: K, value: V) { self.data.push((key, value)); }
    fn load(&self, key: &K) -> Option<&V> { self.data.iter().find(|(k, _)| k == key).map(|(_, v)| v) }
}

impl<K: Eq + Hash, V> Storage<K, V> for HashStorage<K, V>
{
    fn store(&mut self, key: K, value: V) { self.data.insert(key, value); }
    fn load(&self, key: &K) -> Option<&V> { self.data.get(key) }
}

// where clause carries the bounds needed by both inner impls
#[enum_dispatcher(Storage<K, V>)]
enum AnyStorage<K, V>
where K: Eq + Hash + PartialEq
{
    Vec(VecStorage<K, V>),
    Hash(HashStorage<K, V>),
}

// Generated:
//   impl<K, V> Storage<K, V> for AnyStorage<K, V>
//   where
//       K: Eq + Hash + PartialEq,
//   {
//       fn store(&mut self, key: K, value: V)  { match self { ... } }
//       fn load(&self, key: &K) -> Option<&V>  { match self { ... } }
//   }

fn case3()
{
    let mut s: AnyStorage<String, i32> = AnyStorage::Hash(HashStorage { data: HashMap::new() });

    s.store("answer".to_string(), 42);
    println!("{:?}", s.load(&"answer".to_string())); // Some(42)
}

// ═════════════════════════════════════════════════════════════════════════════
// CASE 4 — Generic trait, NON-generic enum with concrete type args
// The enum is locked to one concrete type combination.
// The macro substitutes T → f32, U → f32 inside the generated method signatures.
// ═════════════════════════════════════════════════════════════════════════════

// Reuses Processor<T, U> from case 2.
// ScaleProcessor<f32> and ClampProcessor<f32> are already concrete.

#[enum_dispatcher(Processor<f32, f32>)]
enum F32Effect
{
    Scale(ScaleProcessor<f32>),
    Clamp(ClampProcessor<f32>),
}

// Generated:
//   impl Processor<f32, f32> for F32Effect {
//       fn process(&mut self, input: f32) -> f32 { match self { ... } }
//       fn peek(&self) -> f32                    { match self { ... } }
//   }

fn case4()
{
    let mut e = F32Effect::Scale(ScaleProcessor { scale: 2.0, last: 0.0 });
    println!("result: {}", e.process(3.0)); // 6.0
}

fn main()
{
    case1();
    case2();
    case3();
    case4();
}
