extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(ProcessorComponent)]
pub fn derive_processor_component(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_processor_component_macro(&ast)
}

fn impl_processor_component_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let vis = &ast.vis;

    let compiled_name = format_ident!("Compiled{}", name);

    let syn::Data::Struct(struct_data) = &ast.data else {
        panic!("Only structs are supported");
    };

    let members = struct_data.fields.members();
    let members2 = struct_data.fields.members();
    let members3 = struct_data.fields.members();

    let gen = quote! {
        #vis struct #compiled_name <'ctx> {
            #(#members: <#members as ProcessorComponent>::CompiledType),*
            _ctx: ::core::marker::PhantomData<&'ctx ()>,
        }

        impl ProcessorComponent for #name {
            type CompiledType<'ctx> = #compiled_name <'ctx>;

            fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
                #(self.#members2.visit(visitor));*
            }

            fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
                #(self.#members3.visit_mut(visitor));*
            }

            fn compile<'ctx>(
                &self,
                processor_id: SoundProcessorId,
                compiler: &mut SoundGraphCompiler<'_, 'ctx>,
            ) -> Self::CompiledType<'ctx> {
                todo!()
            }
        }
    };
    gen.into()
}
