#[macro_export]
macro_rules! pprint_ast {
    ( $arg:expr ) => {
        quote::ToTokens::into_token_stream($arg).to_string()
    };
}

#[macro_export]
macro_rules! CHRusty_todo {
    ( message! $message:expr, $( $args:expr ),* ) => {
        {
            #[cfg(debug_assertions)]
            panic!($message, $($args)*)
        }
    };
    ( recover! $else:expr,
      justification! $justification:expr  ,
      message! $message:expr ,  $( $args:expr ),* ) => {
        {
            if cfg!(debug_assertions) {
                panic!($message, $( $args, ),*)
            } else {
                eprintln!($justification, $($args)*);
                $else
            }
        }
    }
}

#[macro_export]
macro_rules! CHRusty_parse {
    ($exp:literal as $ty:ty) => {
        syn::parse_str::<$ty>($exp).unwrap()
    };
    ($exp:ident as $ty:ty) => {
        syn::parse_str::<$ty>($exp).unwrap()
    };
    (($exp:expr) as $ty:ty) => {
        syn::parse_str::<$ty>($exp).unwrap()
    };
}

#[macro_export]
macro_rules! CHRusty_build {
    ($name:path { $( $field:ident: $field_value:expr ),*; default![$($default_field:ident),*] }) => {
        {$name {
            $( $field: $field_value ),*,
            $( $default_field: Default::default() ),*
        }}
    };
    ($name:path { $( $field:ident: $field_value:expr ),* }) => {
        {$name {
            $( $field: $field_value ),*
        }}
    }
}
