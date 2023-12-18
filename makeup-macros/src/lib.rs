use quote::quote;

#[doc(hidden)]
#[proc_macro]
pub fn __do_check_mail_arms(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<syn::Expr>(input).expect("Failed to parse input");

    let mut left_patterns = Vec::new();
    let mut right_patterns = Vec::new();
    let mut left_arms = Vec::new();
    let mut right_arms = Vec::new();

    if let syn::Expr::Group(group) = input {
        if let syn::Expr::Match(expr_match) = group.expr.as_ref() {
            for arm in expr_match.arms.iter() {
                let pattern = arm.pat.clone();
                let handler = arm.body.clone();

                // If pattern is an ident starting with MakeupMessage::, put it in the right arm.
                // Otherwise, put it in the left arm.

                let is_makeup_message = match &pattern {
                    syn::Pat::TupleStruct(ref pat_tuple_struct) => pat_tuple_struct.path.segments
                        [0]
                    .ident
                    .to_string()
                    .starts_with("MakeupMessage"),
                    _ => false,
                };

                if is_makeup_message {
                    right_patterns.push(pattern.clone());
                    right_arms.push(handler);
                } else {
                    left_patterns.push(pattern.clone());
                    left_arms.push(handler);
                }
            }
        }
    }
    let output = quote! {
        match message {
            #(Either::Left(#left_patterns) => #left_arms)*
            #(Either::Right(#right_patterns) => #right_arms)*
            _ => {}
        }
    };

    output.into()
}
