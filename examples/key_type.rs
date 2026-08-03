#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum OrderType
{
    Sell,
    Buy,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Sell;
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Buy;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Order<T>
{
    order_type: T,
    price:      f64,
    quantity:   f64,
}

impl<T> Order<T>
{
    fn with_type<U>(self, order_type: U) -> Order<U> { Order { order_type, price: self.price, quantity: self.quantity } }
}

#[derive(Clone, Default, Debug)]
struct Engine
{
    asks: Vec<Order<Sell>>,
    bids: Vec<Order<Buy>>,
}

impl Engine
{
    fn place(&mut self, order: Order<OrderType>)
    {
        match order.order_type
        {
            OrderType::Sell => self.asks.push(order.with_type(Sell)),
            OrderType::Buy => self.bids.push(order.with_type(Buy)),
        }
    }
}

fn main()
{
    let mut engine = Engine::default();

    engine.place(Order {
        order_type: OrderType::Sell,
        price:      10.0,
        quantity:   10.0,
    });
    engine.place(Order {
        order_type: OrderType::Buy,
        price:      5.0,
        quantity:   10.0,
    });

    dbg!(engine);
}
