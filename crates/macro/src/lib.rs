use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    BareFnArg,
    BareVariadic,
    Ident,
    ItemFn,
    PatType,
    TypeBareFn,
    parse_macro_input,
    parse_quote,
    punctuated::Punctuated,
    token::Comma,
};

#[proc_macro_attribute]
pub fn static_hook(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func_item: ItemFn = parse_macro_input!(item);
    let vis = func_item.vis.clone();
    let name = func_item.sig.ident.clone();
    let unsafety = func_item.sig.unsafety;
    let abi = func_item.sig.abi.clone();
    let ty_args = func_item
        .sig
        .inputs
        .iter()
        .map::<BareFnArg, _>(|arg| parse_quote!(#arg))
        .collect();
    let variadic = func_item.sig.variadic.clone().map(|var| BareVariadic {
        attrs: var.attrs.clone(),
        name: Default::default(),
        dots: Default::default(),
        comma: var.comma,
    });
    let output = func_item.sig.output.clone();
    let ty_func = TypeBareFn {
        lifetimes: Default::default(),
        unsafety,
        abi,
        fn_token: Default::default(),
        paren_token: Default::default(),
        inputs: ty_args,
        variadic,
        output,
    };
    let pfn_ident = Ident::new(&format!("PFN_{name}"), Span::call_site());
    let hook_ident = Ident::new(&format!("Hook_{name}"), Span::call_site());
    let init_ident = Ident::new(&format!("init_{name}"), Span::call_site());
    let detour_ident = Ident::new(&format!("my_{name}"), Span::call_site());
    let orig_ident = Ident::new(&format!("orig_{name}"), Span::call_site());
    let mut renamed_detour = func_item.clone();
    renamed_detour.sig.ident = detour_ident.clone();
    let mut call_orig_func = func_item.clone();
    call_orig_func.sig.ident = orig_ident;
    let val_args: Punctuated<_, Comma> = func_item
        .sig
        .inputs
        .iter()
        .map::<PatType, _>(|arg| parse_quote!(#arg))
        .map(|t| t.pat)
        .collect();
    call_orig_func.block = parse_quote! {{
        #hook_ident.wait().call(#val_args)
    }};
    let o = quote! {
        #vis type #pfn_ident = #ty_func;
        #vis static #hook_ident: ::std::sync::OnceLock<
            ::retour::GenericDetour<
            #pfn_ident
        >> = ::std::sync::OnceLock::new();
        #renamed_detour
        #call_orig_func
        #vis unsafe fn #init_ident(
            f: #pfn_ident
        ) -> ::std::result::Result<
            &'static ::retour::GenericDetour<
            #pfn_ident
        >, ::retour::Error> {
            let a = unsafe {
                ::retour::GenericDetour::new(f, #detour_ident)
            }?;
            ::std::result::Result::Ok(#hook_ident.get_or_init(|| a))
        }
    };
    o.into()
}

#[proc_macro_attribute]
pub fn gen_pfn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func_item: ItemFn = parse_macro_input!(item);
    let vis = func_item.vis.clone();
    let name = func_item.sig.ident.clone();
    let unsafety = func_item.sig.unsafety;
    let abi = func_item.sig.abi.clone();
    let ty_args = func_item
        .sig
        .inputs
        .iter()
        .map::<BareFnArg, _>(|arg| parse_quote!(#arg))
        .collect();
    let variadic = func_item.sig.variadic.clone().map(|var| BareVariadic {
        attrs: var.attrs.clone(),
        name: Default::default(),
        dots: Default::default(),
        comma: var.comma,
    });
    let output = func_item.sig.output.clone();
    let ty_func = TypeBareFn {
        lifetimes: Default::default(),
        unsafety,
        abi,
        fn_token: Default::default(),
        paren_token: Default::default(),
        inputs: ty_args,
        variadic,
        output,
    };
    let pfn_ident = Ident::new(&format!("PFN_{name}"), Span::call_site());
    let detour_ident = Ident::new(&format!("my_{name}"), Span::call_site());
    let mut renamed_detour = func_item.clone();
    renamed_detour.sig.ident = detour_ident.clone();
    let o = quote! {
        #vis type #pfn_ident = #ty_func;
        #renamed_detour
    };
    o.into()
}
