pub trait TState<TData>: std::fmt::Debug
{
    fn is_valid(&self, data: &mut TData) -> bool;
}
/// # Xynok Linear State Machine
/// Each state has only one target to transit to.
/// If your machine needs multiple state branches(a state may has mutiple branches pointing to
/// anothers). Please split them into smaller states and transit them one by one.
/// > This design forces your design must clear, simple to read and explicity.
/// ## Documentation supporting
/// This macro uses [procedure comment for properties](https://amanjeev.com/blog/rust-document-macro-invocations/).
/// Your comment must start with docs token:
/// ```rust
/// /// your comment here ... start with `///` three slashes.
/// ```
#[macro_export]
macro_rules! state_machine {
    (
        $(#[$machine_cmt:meta])*
        machine: $machine_name:ident,
        machine_data_type: $machine_data:ty,
        state_group: $state_group:ident,
        states: [
        $($(#[$state_cmt:meta])* $state:ident),+ $(,)?
        ],
        initial_state: $initial_state:ident,
        // these precheck states will be checked after current state completed and before its transition
        precheck_states: [$($precheck_state:ident),*],
        transitions: [
        $($(#[$transition_cmt:meta])* $from:ident -> $to:ident),+ $(,)?
        ]
    ) => {

        $(
            $(#[$state_cmt])*
            #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
            pub struct $state;
        )+

        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum $state_group {
            $(
                $(#[$state_cmt])*
                $state($state),
            )+
        }


        impl TState<$machine_data> for $state_group
        {
            fn is_valid(&self, data: &mut $machine_data) -> bool
            {
                // match all states, not just transition sources, because some state dont have its
                // transition, this make an infinity loop, even with panic!() in transit()
                match self
                {
                    $(Self::$state(state) => state.is_valid(data),)+
                }
            }
        }
        impl $state_group
        {
            fn transit(&self) -> Self
            {
                match self
                {

                    $(
                        $(#[$transition_cmt])*
                        Self::$from(_) => Self::$to($to),
                    )+
                    _ => panic!("❌ no transition defined for state `{:?}`!", self)
                }
            }
        }

        $(#[$machine_cmt])*
        #[allow(non_snake_case)]
        pub struct $machine_name
        {
            data: $machine_data,
            current_state: $state_group,
            must_precheck: bool,
            $($precheck_state: $state_group,)*
        }

        impl $machine_name
        {
            pub fn new(machine_data: $machine_data) -> Self
            {
                Self
                {
                    data: machine_data,
                    current_state: $state_group::$initial_state($initial_state),
                    must_precheck: true,
                    $($precheck_state: $state_group::$precheck_state($precheck_state),)*
                }
            }
            pub fn data(&self) -> &$machine_data
            {
                &self.data
            }


            pub fn current_state(&self) -> &$state_group
            {
                &self.current_state
            }

            pub fn update(&mut self)
            {
                if self.must_precheck
                {

                    $(
                        if self.$precheck_state.is_valid(&mut self.data)
                        {
                            self.current_state = self.$precheck_state.transit();
                            self.must_precheck = false;
                            return;
                        }
                    )*
                }
                self.must_precheck = true;
                if !self.current_state.is_valid(&mut self.data) { return; }
                self.current_state = self.current_state.transit();
            }

        }
    };
}
