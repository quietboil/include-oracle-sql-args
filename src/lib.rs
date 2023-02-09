//! This crate provides helper proc-macro(s) that assists [include-oracle-sql][1] in generating Yesql-like methods for using SQL in Rust.
//!
//! [1]: https://github.com/quietboil/include-oracle-sql

use proc_macro;
use proc_macro2::{TokenStream, Literal, Group, Delimiter, Punct, Spacing};
use syn::{Token, parse::{Parse, ParseStream}};
use quote::TokenStreamExt;

/**
Returns the uppercase equivalent of this identifier as a string literal.

```
let as_literal = include_oracle_sql_args::to_uppercase!(param_name);
assert_eq!(as_literal, "PARAM_NAME");
```
*/
#[proc_macro]
pub fn to_uppercase(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let in_arg = syn::parse_macro_input!(input as syn::Ident);
    let in_name = in_arg.to_string();
    let out_name = in_name.to_uppercase();
    let mut tokens = TokenStream::new();
    tokens.append(Literal::string(&out_name));
    tokens.into()
}

/**
Maps arguments of the generated database access method into a tuple of SQL arguments.

## Examples

### Single Argument

```
let arg = 42;
let args = include_oracle_sql_args::map!(arg => "SELECT " :arg " FROM xxx WHERE x = " :arg " OR y = " :arg " ORDER BY z");
assert_eq!(args, 42);
```

### Exactly 2 Parameters

```
let a1 = 27;
let a2 = "name";
let args = include_oracle_sql_args::map!(a1 a2 => "SELECT * FROM xxx WHERE a = " :a1 " AND b = " :a2);
assert_eq!(args, (27, "name", ()));
```

### Unique SQL Parameters

```
let a1 = 31;
let a2 = "text";
let a3 = &["a", "b", "c"];
let args = include_oracle_sql_args::map!(a1 a2 a3 => "UPDATE xxx SET a = " :a1 ", :b = " :a2 " WHERE c IN (" #a3 ")");
assert_eq!(args, (31, "text", &["a", "b", "c"]));
```

### Duplicate SQL Parameters

```
let id = 19;
let name = "unknown";
let data = 3.14;
let args = include_oracle_sql_args::map!(id name data => "UPDATE xxx SET a = " :name ", b = " :name ", c = " :data " WHERE i = " :id " OR ( x = " :name " AND i != " :id ")");
assert_eq!(args, (
    ("ID",   19),
    ("NAME", "unknown"),
    ("DATA", 3.14),
));
```

### Reordered SQL Parameters

```
let a1 = 31;
let a2 = "text";
let a3 = &["a", "b", "c"];
let args = include_oracle_sql_args::map!(a1 a2 a3 => "UPDATE xxx SET a = " :a2 ", :b = " :a1 " WHERE c IN (" #a3 ")");
assert_eq!(args, (
    ("A1", 31), 
    ("A2", "text"),
    ("A3", &["a", "b", "c"]),
));
```

### Mutable (OUT) Argument

```
let id = 101;
let mut name = String::new();
let out_name = &mut name;
let args = include_oracle_sql_args::map!(id out_name => "UPDATE xxx SET x = x || 'X' WHERE i = " :id " RETURN x INTO " :out_name);
assert_eq!(args.0, 101);
assert_eq!(args.2, ());
// emulate output
args.1.push_str("TestX");
assert_eq!(name, "TestX");
```
*/
#[proc_macro]
pub fn map(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let MapArgs { mut method_args, sql_args } = syn::parse_macro_input!(input as MapArgs);

    let mut tokens = TokenStream::new();

    if method_args.len() == 1 {
        tokens.append(method_args.remove(0));
    } else if method_args == sql_args {
        let mut items = TokenStream::new();
        if method_args.len() == 2 {
            items.append_terminated(method_args, Punct::new(',', Spacing::Alone));
            let unit = TokenStream::new();
            items.append(Group::new(Delimiter::Parenthesis, unit));
        } else {
            items.append_separated(method_args, Punct::new(',', Spacing::Alone));
        }
        tokens.append(Group::new(Delimiter::Parenthesis, items));
    } else {
        let mut items = TokenStream::new();
        for arg in method_args {
            if !items.is_empty() {
                items.append(Punct::new(',', Spacing::Alone));
            }
            let mut name_value = TokenStream::new();
            name_value.append(Literal::string(arg.to_string().to_uppercase().as_str()));
            name_value.append(Punct::new(',', Spacing::Alone));
            name_value.append(arg);
            items.append(Group::new(Delimiter::Parenthesis, name_value))
        }
        tokens.append(Group::new(Delimiter::Parenthesis, items));
    }
    tokens.into()
}

struct MapArgs {
    method_args: Vec<syn::Ident>,
    sql_args: Vec<syn::Ident>,
}

impl Parse for MapArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut method_args = Vec::new();
        while !input.peek(Token![=>]) {
            let arg = input.parse()?;
            method_args.push(arg);
        }
        let _ : Token![=>] = input.parse()?;
        let mut sql_args = Vec::new();
        while !input.is_empty() {
            let _ : syn::LitStr = input.parse()?;
            if input.is_empty() {
                break;
            }
            let variant = input.lookahead1();
            if variant.peek(Token![:]) {
                let _ : Token![:] = input.parse()?;
            } else if variant.peek(Token![#]) {
                let _ : Token![#] = input.parse()?;
            } else {
                return Err(variant.error());
            }
            let arg = input.parse()?;
            sql_args.push(arg);
        }
        Ok(Self { method_args, sql_args })
    }
}
