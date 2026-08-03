#![allow(unused)]
use xynok_std::Binder;

use std::{sync::Arc, thread};

fn hello(some_value: &i32)
{
    println!("hello value: {}", some_value);
}
fn goodbye(some_value: &i32)
{
    println!("goodbye value: {}", some_value);
}

struct Agent {}

impl Agent
{
    fn hello(&self, some_value: &i32)
    {
        println!("Agent hello value: {}", some_value);
    }
    fn goodbye(some_value: &i32)
    {
        println!("Agent goodbye value: {}", some_value);
    }
}
fn main()
{
    let binder = Binder::new(0i32);
    let agent = Arc::new(Agent {});

    let id_hello = binder.add_listener(hello);
    let id_agent_hello = binder.add_listener({
        let agent = Arc::clone(&agent);
        move |v| agent.hello(v)
    });
    let id_goodbye = binder.add_listener(goodbye);

    binder.set(42);

    binder.remove_listener(id_agent_hello);

    let agent_val = 1000i32;
    agent.hello(&agent_val); // still valid — Arc keeps it alive

    binder.set(99);
}
// hello, goodbye}
