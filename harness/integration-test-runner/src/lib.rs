extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Ident, ItemFn};

/// 매크로 인자를 나타내는 구조체
struct RunInArgs {
    mode: Ident,
}

impl Parse for RunInArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mode: Ident = input.parse()?;
        Ok(RunInArgs { mode })
    }
}

#[proc_macro_attribute]
pub fn run_in_(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as RunInArgs);
    let mode = args.mode.to_string();

    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as ItemFn);

    // Extract the function's attributes, visibility, signature, and body
    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let fn_name = &sig.ident;
    let asyncness = &sig.asyncness;

    // Ensure the test function is async
    if asyncness.is_none() {
        return syn::Error::new_spanned(
            sig.fn_token,
            "The #[run_in_(container|local)] attribute can only be applied to async functions",
        )
        .to_compile_error()
        .into();
    }

    let expanded = match mode.as_str() {
        "container" => {
            quote! {
                #(#attrs)*
                #vis #sig {
                    use harness::utils::is_running_in_container;
                    async fn run_test() #block

                    if is_running_in_container() {
                        run_test().await
                    } else {
                        use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};
                        use testcontainers::{
                            core::{client, wait::ExitWaitStrategy, Mount},
                            ImageExt,
                        };
                        use tokio::{
                            io::{AsyncBufRead, AsyncBufReadExt},
                        };

                        use tokio::sync::mpsc;
                        use tokio::time::Duration;
                        use harness::utils::{get_exit_code, read_stdout};

                        let container = GenericImage::new("raftify", "latest")
                            .with_wait_for(WaitFor::exit(ExitWaitStrategy::new()))
                            .with_startup_timeout(Duration::from_secs(6000000))
                            .with_env_var("TEST_NAME", stringify!(#fn_name))
                            .with_container_name(stringify!(#fn_name))
                            .start()
                            .await
                            .expect("Failed to start container");

                        let cid = container.id();
                        let exit_code = get_exit_code(&cid).expect("Failed to get exit code");

                        read_stdout(container.stdout(true)).await;
                        read_stdout(container.stderr(true)).await;

                        container.rm().await.expect("Failed to remove container");

                        assert_eq!(exit_code, 0);
                    }
                }
            }
        }
        "local" => {
            quote! {
                #(#attrs)*
                #vis #sig {
                    println!("TODO: Running test in local mode.");
                }
            }
        }
        _ => {
            return syn::Error::new_spanned(
                args.mode,
                "Invalid argument. Expected 'container' or 'local'",
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(expanded)
}
