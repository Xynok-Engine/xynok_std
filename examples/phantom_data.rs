use std::marker::PhantomData;

trait Log
{
    fn log(&self);
}

struct Door<T>
where T: Log
{
    state: PhantomData<T>,
}

struct Locked {}
struct Unlocked {}

impl Log for Locked
{
    fn log(&self)
    {
        println!("this is a locked door!");
    }
}

impl Log for Unlocked
{
    fn log(&self)
    {
        println!("this is an unlocked door!");
    }
}

impl Door<Locked>
{
    fn open(&self)
    {
        Locked {}.log();
        println!("door is opening!");
    }
}

impl Door<Unlocked>
{
    fn close(&self)
    {
        Unlocked {}.log();
        println!("door is closing!");
    }
}

fn main()
{
    println!("Hello, this is xynok std phantom data practice examples!");
    let unlocked = Door::<Unlocked> { state: PhantomData };
    let locked = Door::<Locked> { state: PhantomData };

    unlocked.close();
    locked.open();
}
