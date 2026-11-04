//! Attribute macro: `#[lariv_rs::main(...)]` for large-stack async entrypoints.

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Expr, ItemFn, LitStr, Result, Token, parse_macro_input, parse_quote};

/// Parsed `#[lariv_rs::main(...)]` arguments.
struct MainArgs {
    stack_size: Option<Expr>,
    flavor: Flavor,
    thread_name: Option<LitStr>,
}

#[derive(Clone, Copy)]
enum Flavor {
    CurrentThread,
    MultiThread,
}

impl Parse for MainArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut stack_size = None;
        let mut flavor = Flavor::CurrentThread;
        let mut thread_name = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match ident.to_string().as_str() {
                "stack_size" => {
                    if stack_size.is_some() {
                        return Err(syn::Error::new(ident.span(), "duplicate `stack_size`"));
                    }
                    stack_size = Some(input.parse()?);
                }
                "flavor" => {
                    let lit: LitStr = input.parse()?;
                    flavor = match lit.value().as_str() {
                        "current_thread" => Flavor::CurrentThread,
                        "multi_thread" => Flavor::MultiThread,
                        other => {
                            return Err(syn::Error::new(
                                lit.span(),
                                format!(
                                    "unknown flavor `{other}`; expected `current_thread` or `multi_thread`"
                                ),
                            ));
                        }
                    };
                }
                "thread_name" => {
                    if thread_name.is_some() {
                        return Err(syn::Error::new(ident.span(), "duplicate `thread_name`"));
                    }
                    thread_name = Some(input.parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown attribute `{other}`; expected `stack_size`, `flavor`, or `thread_name`"
                        ),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            stack_size,
            flavor,
            thread_name,
        })
    }
}

pub fn main_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MainArgs);
    let mut input_fn = parse_macro_input!(item as ItemFn);

    if input_fn.sig.asyncness.is_none() {
        return syn::Error::new(
            input_fn.sig.span(),
            "`#[lariv_rs::main]` must be applied to an `async fn`",
        )
        .to_compile_error()
        .into();
    }

    if input_fn.sig.ident != "main" {
        return syn::Error::new(
            input_fn.sig.ident.span(),
            "`#[lariv_rs::main]` must be applied to a function named `main`",
        )
        .to_compile_error()
        .into();
    }

    // Rename the async body so we can emit a sync `fn main`.
    input_fn.sig.ident = parse_quote!(__lariv_async_main);
    input_fn.attrs.push(parse_quote!(#[doc(hidden)]));

    let stack_size = args
        .stack_size
        .unwrap_or_else(|| parse_quote!(64 * 1024 * 1024));
    let thread_name = args
        .thread_name
        .unwrap_or_else(|| LitStr::new("lariv-server", proc_macro2::Span::call_site()));

    let runtime_builder = match args.flavor {
        Flavor::CurrentThread => quote! {
            ::tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime")
        },
        Flavor::MultiThread => quote! {
            ::tokio::runtime::Builder::new_multi_thread()
                .thread_stack_size(__LARIV_STACK_SIZE)
                .enable_all()
                .build()
                .expect("tokio runtime")
        },
    };

    let main_fn = quote_spanned! {input_fn.sig.span()=>
        fn main() {
            const __LARIV_STACK_SIZE: usize = #stack_size;
            __lariv_raise_process_stack_limit(__LARIV_STACK_SIZE);

            let __lariv_join = ::std::thread::Builder::new()
                .name(::std::string::String::from(#thread_name))
                .stack_size(__LARIV_STACK_SIZE)
                .spawn(|| {
                    #runtime_builder.block_on(__lariv_async_main())
                })
                .expect("spawn server thread")
                .join();

            match __lariv_join {
                ::std::result::Result::Ok(::std::result::Result::Ok(())) => {}
                ::std::result::Result::Ok(::std::result::Result::Err(e)) => {
                    ::std::eprintln!("{e:#}");
                    ::std::process::exit(1);
                }
                ::std::result::Result::Err(_) => {
                    ::std::eprintln!("server thread panicked");
                    ::std::process::exit(1);
                }
            }
        }

        /// Raise the process stack soft limit so `thread::Builder::stack_size` is not clipped
        /// by `ulimit -s` (often 8192 KiB).
        #[cfg(unix)]
        fn __lariv_raise_process_stack_limit(bytes: usize) {
            #[repr(C)]
            struct Rlimit {
                rlim_cur: u64,
                rlim_max: u64,
            }
            unsafe extern "C" {
                fn getrlimit(resource: i32, rlim: *mut Rlimit) -> i32;
                fn setrlimit(resource: i32, rlim: *const Rlimit) -> i32;
            }
            const RLIMIT_STACK: i32 = 3;
            const RLIM_INFINITY: u64 = u64::MAX;

            let mut lim = ::std::mem::MaybeUninit::<Rlimit>::uninit();
            // SAFETY: `lim` is a valid `rlimit`-sized buffer for `getrlimit`.
            let rc = unsafe { getrlimit(RLIMIT_STACK, lim.as_mut_ptr()) };
            if rc != 0 {
                return;
            }
            // SAFETY: `getrlimit` succeeded, so `lim` is initialized.
            let mut lim = unsafe { lim.assume_init() };
            let want = bytes as u64;
            lim.rlim_cur = if lim.rlim_max == RLIM_INFINITY {
                want
            } else {
                want.min(lim.rlim_max)
            };
            // SAFETY: `lim` is a fully initialized `rlimit`.
            let rc = unsafe { setrlimit(RLIMIT_STACK, &lim) };
            if rc != 0 {
                ::std::eprintln!(
                    "warning: could not raise stack limit to {} MiB; \
                     try `ulimit -s unlimited` before starting the server",
                    bytes / (1024 * 1024)
                );
            }
        }

        #[cfg(not(unix))]
        fn __lariv_raise_process_stack_limit(_bytes: usize) {}
    };

    TokenStream::from(quote! {
        #input_fn
        #main_fn
    })
}
