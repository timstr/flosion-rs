extern crate proc_macro;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(ProcessorComponents)]
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

    let fields_with_compiled_types: Vec<TokenStream> = named_fields
        .named
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

    let members1 = struct_data.fields.members();
    let members2 = struct_data.fields.members();
    let members3 = struct_data.fields.members();
    let members4 = struct_data.fields.members();

    let gen = quote! {
        #vis struct #compiled_name <'ctx> {
            #(#fields_with_compiled_types,)*
            _ctx: ::core::marker::PhantomData<&'ctx ()>,
        }

        impl ::flosion::core::sound::soundprocessor::ProcessorComponent for #name {
            type CompiledType<'ctx> = #compiled_name <'ctx>;

            fn visit<'a>(&self, visitor: &'a mut dyn ::flosion::core::sound::soundprocessor::ProcessorComponentVisitor) {
                #(self.#members2.visit(visitor));*
            }

            fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ::flosion::core::sound::soundprocessor::ProcessorComponentVisitorMut) {
                #(self.#members3.visit_mut(visitor));*
            }

            fn compile<'ctx>(
                &self,
                processor_id: ::flosion::core::sound::soundprocessor::SoundProcessorId,
                compiler: &mut ::flosion::core::engine::soundgraphcompiler::SoundGraphCompiler<'_, 'ctx>,
            ) -> Self::CompiledType<'ctx> {
                #compiled_name {
                    #(#members1 : self.#members1.compile(processor_id, compiler),)*
                    _ctx: ::core::marker::PhantomData
                }
            }
        }

        impl<'ctx> ::flosion::core::sound::soundprocessor::StartOver for #compiled_name <'ctx> {
            fn start_over(&mut self) {
                #(self.#members4.start_over());*
            }
        }
    };

    TokenStream::from(gen)
}
