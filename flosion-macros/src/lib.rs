extern crate proc_macro;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(ProcessorComponents, attributes(not_a_component, state))]
pub fn derive_processor_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_processor_component_macro(&ast).into()
}

fn impl_processor_component_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let vis = &ast.vis;

    let compiled_name = format_ident!("Compiled{}", name);

    let syn::Data::Struct(struct_data) = &ast.data else {
        panic!("Only structs are supported");
    };

    let syn::Fields::Named(named_fields) = &struct_data.fields else {
        panic!("Only structs with named fields are supported");
    };

    let component_fields: Vec<syn::Field> = named_fields
        .named
        .iter()
        .filter_map(|f| {
            if f.attrs.iter().any(|attr| match attr.path().get_ident() {
                Some(name) => name == "not_a_component" || name == "state",
                None => false,
            }) {
                return None;
            }
            Some(f.clone())
        })
        .collect();

    let component_field_names: Vec<proc_macro2::Ident> = component_fields
        .iter()
        .map(|f| f.ident.as_ref().unwrap().clone())
        .collect();

    let component_fields_type_decls: Vec<TokenStream> = component_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            let gen = quote! {
                #ident : <#ty as ::flosion::core::sound::soundprocessor::ProcessorComponent>::CompiledType<'ctx>
            };

            TokenStream::from(gen)
        })
        .collect();

    let state_fields_and_inner_types: Vec<(syn::Field, syn::Type)> = named_fields
        .named
        .iter()
        .filter_map(|f| {
            if f.attrs.iter().any(|attr| match attr.path().get_ident() {
                Some(name) => name == "state",
                None => false,
            }) {
                let ty = &f.ty;
                let syn::Type::Path(path) = ty else {
                    panic!("Fields marked with #[state] must have type StateMarker<T>");
                };
                assert!(path.qself.is_none());
                let last_path_segment = path.path.segments.last().unwrap();
                assert!(last_path_segment.ident == "StateMarker");
                let syn::PathArguments::AngleBracketed(args) = &last_path_segment.arguments else {
                    panic!("Fields marked with #[state] must have type StateMarker<T>");
                };

                assert!(args.args.len() == 1);

                let syn::GenericArgument::Type(inner_type) = args.args.first().unwrap() else {
                    panic!("Fields marked with #[state] must have type StateMarker<T>");
                };

                return Some((f.clone(), inner_type.clone()));
            }
            None
        })
        .collect();

    let state_field_type_decls: Vec<TokenStream> = state_fields_and_inner_types
        .iter()
        .map(|(f, inner_type)| {
            let ident = &f.ident;

            let gen = quote! {
                #ident: #inner_type
            };

            TokenStream::from(gen)
        })
        .collect();

    let state_field_type_inits: Vec<TokenStream> = state_fields_and_inner_types
        .iter()
        .map(|(f, inner_type)| {
            let ident = &f.ident;

            let gen = quote! {
                #ident: #inner_type::new(self)
            };

            TokenStream::from(gen)
        })
        .collect();

    let state_field_names: Vec<syn::Ident> = state_fields_and_inner_types
        .iter()
        .map(|(f, _)| f.ident.as_ref().unwrap().clone())
        .collect();

    let gen = quote! {
        #vis struct #compiled_name <'ctx> {
            #(#component_fields_type_decls,)*
            #(#state_field_type_decls,)*
            _ctx: ::core::marker::PhantomData<&'ctx ()>,
        }

        impl ::flosion::core::sound::soundprocessor::ProcessorComponent for #name {
            type CompiledType<'ctx> = #compiled_name <'ctx>;

            fn visit<'a>(&self, visitor: &'a mut dyn ::flosion::core::sound::soundprocessor::ProcessorComponentVisitor) {
                #(self.#component_field_names.visit(visitor);)*
            }

            fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ::flosion::core::sound::soundprocessor::ProcessorComponentVisitorMut) {
                #(self.#component_field_names.visit_mut(visitor);)*
            }

            fn compile<'ctx>(
                &self,
                processor_id: ::flosion::core::sound::soundprocessor::SoundProcessorId,
                compiler: &mut ::flosion::core::engine::soundgraphcompiler::SoundGraphCompiler<'_, 'ctx>,
            ) -> Self::CompiledType<'ctx> {
                #compiled_name {
                    #(#component_field_names : self.#component_field_names.compile(processor_id, compiler),)*
                    #(#state_field_type_inits,)*
                    _ctx: ::core::marker::PhantomData
                }
            }
        }

        impl<'ctx> ::flosion::core::sound::soundprocessor::StartOver for #compiled_name <'ctx> {
            fn start_over(&mut self) {
                #(self.#component_field_names.start_over();)*
                #(self.#state_field_names.start_over();)*
            }
        }
    };

    TokenStream::from(gen)
}
