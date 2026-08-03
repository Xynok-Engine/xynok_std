#[macro_export]
macro_rules! heap_define {
    (
        $(#[$meta_root:meta])*
        $app_name:ident =>

        $(#[$meta_heap_ref:meta])*
        $handler_name:ident,

        $(
            $(#[$meta:meta])*
           $accessor:vis $field:ident : $field_type:ty
        ),* $(,)?
    ) =>
    {
        $(#[$meta_root])*
        pub struct $app_name
        {
            $(
                $(#[$meta])*
                $field: xynok_std::RawPtr<$field_type>,
            )*
        }

        $(#[$meta_heap_ref])*
        #[allow(unused)]
        #[derive(Clone, Copy)]
        pub struct $handler_name
        {
            $(
                $(#[$meta])*
                $accessor $field: xynok_std::RawRefMut<$field_type>,
            )*
        }

        impl $app_name
        {
            pub fn as_ref_mut(&self) -> $handler_name
            {
                $handler_name
                {
                    $(
                        $field: self.$field.as_ref_mut(),
                    )*
                }
            }
        }
    };
}
