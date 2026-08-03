#![allow(unused)]
use std::any::{Any, TypeId};

struct SortedHashMap<TKey, TVal>
{
    pub(crate) keys:   Vec<TKey>,
    pub(crate) values: Vec<TVal>,
}

fn add_sort<TKey, TVal>(dst_key: &mut [TKey], dst_values: &mut [TVal])
where TKey: PartialEq
{
}

fn b() {}
fn a() {}
fn main()
{
    let type_a = a.type_id();
    let type_b = b.type_id();
    let val = type_a > type_b;
    let val2 = type_a < type_b;
    let val3 = type_a == type_b;
    println!(">{}, <{}, =={}", val, val2, val3);
}
