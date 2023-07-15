// Write a proc macro that takes in code that looks like the following:
//
// check_mail!(self, ctx, {
//     MakeupMessage::TextUpdate(text) => { ... }
//     MakeupMessage::Tick(_) => { ... }
//     MyMessage::Foo => { ... }
// });
//
// and produces code like the following:
//
// if let Some(mailbox) = ctx.post_office.mailbox(self) {
//     for message in mailbox.iter() {
//         match message {
//             Either::Left(MyMessage::Foo) => { ... }
//             Either::Right(MakeupMessage::TextUpdate(text)) => { ... }
//             Either::Right(MakeupMessage::Tick(_)) => { ... }
//         }
//     }
// }
//
// That is, it should:
// - Take in a list of patterns and handlers
// - If the pattern starts with the token `MakeupMessage`, it should be placed in the right-hand
//   side of the match expression.
// - If the pattern does not start with the token `MakeupMessage`, it should be placed in the
//   left-hand side of the match expression.
//
// And it MUST:
// - Check the first token
// - NOT put the pattern into both sides of the expr
// THIS CODE MUST NOT JUST REPEAT THE EXAMPLE GIVEN.

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
                    syn::Pat::Ident(ref pat_ident) => {
                        pat_ident.ident.to_string().starts_with("MakeupMessage")
                    }
                    _ => false,
                };

                if is_makeup_message {
                    left_patterns.push(pattern.clone());
                    left_arms.push(handler);
                } else {
                    right_patterns.push(pattern.clone());
                    right_arms.push(handler);
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
